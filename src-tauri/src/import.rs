//! Importação de arquivos de e-mail (US15): examina o arquivo, cria o arquivo local e grava em
//! lotes, em segundo plano, com progresso, "Cancelar" e retomada depois de fechar o app (D5). Nada
//! vai para a rede. Logs: só contagens e tipos de falha; nunca o caminho, nomes de pastas ou conteúdo.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mail_parser::MessageParser;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::archive_store::{self, ArchiveRow, Imported, NewArchive};
use crate::error::AppError;
use crate::mail_import::{self, Format, Item, Scan, SourceFolder};
use crate::session::AppState;
use crate::store::{self, Account};

/// Mensagens por transação: o bastante para ser rápido, pouco para a tela não esperar o banco.
const BATCH: usize = 200;
const BATCH_BYTES: usize = 32 * 1024 * 1024;
/// Margem que fica livre no disco (o Windows e o próprio banco precisam de espaço, D6).
pub const DISK_MARGIN: u64 = 1024 * 1024 * 1024;

#[derive(Default)]
pub struct Importer {
    running: Mutex<Option<(String, Arc<AtomicBool>)>>,
    scanning: Mutex<Option<Arc<AtomicBool>>>,
    /// O último exame, para não ler um Takeout de vários GB de novo ao confirmar.
    last_scan: Mutex<Option<(PathBuf, Scan)>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanView {
    #[serde(flatten)]
    pub scan: Scan,
    /// Espaço livre no disco do banco (`None` se o sistema não disser).
    pub free_bytes: Option<u64>,
    pub margin_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    account_id: String,
    imported: u32,
    duplicates: u32,
    failed: u32,
    estimate: u32,
    /// Pasta da última mensagem gravada.
    folder: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Finished {
    archive: ArchiveRow,
    /// "cancelled", "no_space", "source_missing" ou "error"; `None` quando terminou.
    stopped: Option<&'static str>,
}

fn data_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok()
}

/// Espaço livre no disco de uma pasta.
#[cfg(windows)]
pub fn free_space(dir: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut free = 0u64;
    // SAFETY: `wide` termina em zero e vive até o fim da chamada; os outros ponteiros podem ser nulos.
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, std::ptr::null_mut(), std::ptr::null_mut()) };
    (ok != 0).then_some(free)
}

#[cfg(unix)]
pub fn free_space(dir: &Path) -> Option<u64> {
    nix::sys::statvfs::statvfs(dir).ok().map(|s| s.blocks_available() as u64 * s.fragment_size() as u64)
}

#[cfg(not(any(windows, unix)))]
pub fn free_space(_dir: &Path) -> Option<u64> {
    None
}

/// Quanto as pastas escolhidas ocupam: no Takeout as pastas se repetem (marcadores), então nunca
/// mais que o arquivo inteiro.
pub fn needed_bytes(scan: &Scan, selected: &HashSet<String>) -> u64 {
    let sum: u64 = scan.folders.iter().filter(|f| selected.contains(&f.key)).map(|f| f.bytes).sum();
    sum.min(scan.bytes)
}

fn source_path(path: &str) -> Result<PathBuf, AppError> {
    let p = PathBuf::from(path);
    if path.is_empty() || !p.is_absolute() {
        return Err(AppError::InvalidInput);
    }
    if !p.exists() {
        return Err(AppError::ImportSourceMissing);
    }
    Ok(p)
}

// ---------------------------------------------------------------- comandos

/// Seletor nativo (CA1): um arquivo de e-mail ou uma pasta (Thunderbird, Apple Mail, Maildir).
#[tauri::command]
pub async fn pick_mail_source(app: AppHandle, folder: bool) -> Result<Option<String>, AppError> {
    use tauri_plugin_dialog::DialogExt;
    let dialog = app.dialog().file().set_title(if folder { "Escolher a pasta de e-mails" } else { "Escolher o arquivo de e-mail" });
    let picked = tauri::async_runtime::spawn_blocking(move || {
        if folder {
            dialog.blocking_pick_folder()
        } else {
            dialog
                .add_filter("Arquivos de e-mail", &["mbox", "pst", "ost", "msg", "eml", "emlx", "mbx"])
                .add_filter("Todos os arquivos", &["*"])
                .blocking_pick_file()
        }
    })
    .await
    .map_err(|_| AppError::Storage)?;
    Ok(picked.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned()))
}

/// Passo 2: formato, pastas, tamanho e espaço livre. Ler um mbox grande leva alguns segundos.
#[tauri::command]
pub async fn scan_mail_source(app: AppHandle, importer: State<'_, Importer>, path: String) -> Result<ScanView, AppError> {
    let source = source_path(&path)?;
    let cancel = Arc::new(AtomicBool::new(false));
    if let Some(previous) = importer.scanning.lock().expect("trava").replace(cancel.clone()) {
        previous.store(true, Ordering::Relaxed);
    }
    let reading = source.clone();
    let scan = tauri::async_runtime::spawn_blocking(move || mail_import::scan(&reading, &cancel)).await.map_err(|_| AppError::Storage)??;
    tracing::info!(format = scan.format.as_str(), folders = scan.folders.len(), messages = scan.estimate, "arquivo de e-mail examinado");
    *importer.last_scan.lock().expect("trava") = Some((source, scan.clone()));
    let free_bytes = data_dir(&app).and_then(|d| free_space(&d));
    Ok(ScanView { scan, free_bytes, margin_bytes: DISK_MARGIN })
}

/// Fechar o passo 2 enquanto o exame corre.
#[tauri::command]
pub fn cancel_mail_scan(importer: State<'_, Importer>) {
    if let Some(cancel) = importer.scanning.lock().expect("trava").take() {
        cancel.store(true, Ordering::Relaxed);
    }
}

/// Passo 3: cria o arquivo local (ou acrescenta a um, `into`) e começa a gravar em segundo plano.
#[tauri::command]
pub async fn start_mail_import(
    app: AppHandle,
    state: State<'_, AppState>,
    importer: State<'_, Importer>,
    path: String,
    name: String,
    selected: Vec<String>,
    into: Option<String>,
) -> Result<Account, AppError> {
    let source = source_path(&path)?;
    if importer.running.lock().expect("trava").is_some() {
        return Err(AppError::ImportBusy);
    }
    let cached = importer.last_scan.lock().expect("trava").clone().filter(|(p, _)| *p == source).map(|(_, s)| s);
    let scan = match cached {
        Some(s) => s,
        None => {
            let reading = source.clone();
            tauri::async_runtime::spawn_blocking(move || mail_import::scan(&reading, &AtomicBool::new(false))).await.map_err(|_| AppError::Storage)??
        }
    };
    let wanted: HashSet<String> = selected.into_iter().collect();
    let folders: Vec<SourceFolder> = scan.folders.iter().filter(|f| wanted.contains(&f.key)).cloned().collect();
    if folders.is_empty() {
        return Err(AppError::InvalidInput);
    }
    if let Some(free) = data_dir(&app).and_then(|d| free_space(&d)) {
        if needed_bytes(&scan, &wanted) + DISK_MARGIN > free {
            return Err(AppError::ImportNoSpace);
        }
    }
    // No Takeout a mesma mensagem está em vários marcadores: o total é o do arquivo.
    let estimate = if scan.format == Format::Takeout { scan.estimate } else { folders.iter().map(|f| f.count).sum() };
    let source_text = source.to_string_lossy().into_owned();
    let (format, source_name) = (scan.format.as_str().to_owned(), scan.name.clone());
    let account = match into {
        Some(id) => {
            let id = crate::commands::checked_account_id(&id)?;
            state
                .db
                .call(move |c| {
                    let account = store::require_account(c, &id)?;
                    if account.status != store::AccountStatus::Local {
                        return Err(AppError::InvalidInput);
                    }
                    let new = NewArchive { name: &name, format: &format, source_name: &source_name, source_path: &source_text, estimate };
                    archive_store::reopen_archive(c, &id, &new, &folders)?;
                    Ok(account)
                })
                .await?
        }
        None => {
            let id = store::new_account_id();
            state
                .db
                .call(move |c| {
                    let new = NewArchive { name: &name, format: &format, source_name: &source_name, source_path: &source_text, estimate };
                    archive_store::create_archive(c, &id, &new, &folders)
                })
                .await?
        }
    };
    state.emit_accounts_changed();
    spawn(&app, &importer, account.id.clone())?;
    Ok(account)
}

#[tauri::command]
pub fn cancel_mail_import(importer: State<'_, Importer>, account_id: String) {
    if let Some((id, cancel)) = importer.running.lock().expect("trava").as_ref() {
        if *id == account_id {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}

/// "Continuar" uma importação interrompida: lê o arquivo de novo e pula o que já entrou.
#[tauri::command]
pub async fn resume_mail_import(app: AppHandle, state: State<'_, AppState>, importer: State<'_, Importer>, account_id: String) -> Result<(), AppError> {
    let id = crate::commands::checked_account_id(&account_id)?;
    let lookup = id.clone();
    let archive = state.db.call(move |c| archive_store::get_archive(c, &lookup)).await?.ok_or(AppError::AccountNotFound)?;
    if !Path::new(&archive.source_path).exists() {
        return Err(AppError::ImportSourceMissing);
    }
    let marking = id.clone();
    state.db.call(move |c| archive_store::set_state(c, &marking, "importing")).await?;
    spawn(&app, &importer, id)
}

/// "Descartar o que entrou": apaga o arquivo local inteiro e troca a chave do banco.
#[tauri::command]
pub async fn remove_archive(state: State<'_, AppState>, importer: State<'_, Importer>, account_id: String) -> Result<(), AppError> {
    let id = crate::commands::checked_account_id(&account_id)?;
    if importer.running.lock().expect("trava").as_ref().is_some_and(|(running, _)| *running == id) {
        return Err(AppError::ImportBusy);
    }
    let lookup = id.clone();
    let account = state.db.call(move |c| store::require_account(c, &lookup)).await?;
    if account.status != store::AccountStatus::Local {
        return Err(AppError::InvalidInput);
    }
    crate::privacy::delete_account_data(&state, id).await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveView {
    #[serde(flatten)]
    pub archive: ArchiveRow,
    pub bytes: u64,
    pub running: bool,
}

/// Configurações → Geral → Arquivos de e-mail, e o aviso de importação interrompida.
#[tauri::command]
pub async fn list_archives(state: State<'_, AppState>, importer: State<'_, Importer>) -> Result<Vec<ArchiveView>, AppError> {
    let running = importer.running.lock().expect("trava").as_ref().map(|(id, _)| id.clone());
    let (archives, sizes) = state.db.call(|c| Ok((archive_store::list_archives(c)?, store::stored_bytes_by_account(c)?))).await?;
    let sizes: HashMap<String, u64> = sizes.into_iter().collect();
    Ok(archives
        .into_iter()
        .map(|mut a| {
            let is_running = running.as_deref() == Some(a.account_id.as_str());
            // Parou sem passar pelo fim (app bloqueado no meio, banco fechado): é uma importação interrompida.
            if a.state == "importing" && !is_running {
                a.state = "interrupted".into();
            }
            ArchiveView { bytes: sizes.get(&a.account_id).copied().unwrap_or(0), running: is_running, archive: a }
        })
        .collect())
}

// ---------------------------------------------------------------- execução

fn spawn(app: &AppHandle, importer: &Importer, account_id: String) -> Result<(), AppError> {
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut running = importer.running.lock().expect("trava");
        if running.is_some() {
            return Err(AppError::ImportBusy);
        }
        *running = Some((account_id.clone(), cancel.clone()));
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let stopped = run(&app, &account_id, cancel.clone()).await;
        *app.state::<Importer>().running.lock().expect("trava") = None;
        let state = app.state::<AppState>();
        let new_state = match stopped {
            None | Some("cancelled") => "done",
            Some(_) => "interrupted",
        };
        let id = account_id.clone();
        let result = state
            .db
            .call(move |c| {
                archive_store::order_by_date(c, &id)?;
                archive_store::set_state(c, &id, new_state)?;
                archive_store::get_archive(c, &id)
            })
            .await;
        match result {
            Ok(Some(archive)) => {
                let failed: u32 = archive.failures.values().sum();
                tracing::info!(imported = archive.imported, duplicates = archive.duplicates, failed, stopped = stopped.unwrap_or("-"), "importação terminou");
                let _ = app.emit("import-finished", Finished { archive, stopped });
            }
            Ok(None) => {}
            Err(e) => tracing::warn!(kind = e.kind(), "importação: estado final não gravado"),
        }
        state.emit_accounts_changed();
    });
    Ok(())
}

/// Interpreta uma mensagem lida (cabeçalhos, anexos, prévia) fora da conexão do banco. `None`: não
/// vai para nenhuma pasta escolhida.
pub(crate) fn prepare(parser: &MessageParser, folder_ids: &HashMap<String, i64>, item: Item) -> Option<Result<Imported, mail_import::Failure>> {
    match item {
        Item::Failed(f) => Some(Err(f)),
        Item::Message { folders, raw, flags } => {
            let targets: Vec<i64> = folders.iter().filter_map(|k| folder_ids.get(k).copied()).collect();
            if targets.is_empty() {
                return None;
            }
            // Sem nenhum cabeçalho que toda mensagem tem, o trecho não é uma mensagem (mbox corrompido).
            let fields = crate::message::parse_header_fields(parser, &raw)
                .filter(|f| !f.from_address.is_empty() || !f.subject.is_empty() || f.date.is_some() || f.message_id.is_some());
            let Some(fields) = fields else { return Some(Err(mail_import::Failure::Unreadable)) };
            Some(Ok(Imported {
                key: archive_store::dedupe_key(&fields, &raw),
                attachments: crate::message::attachments_of(&raw),
                preview: crate::preview::from_raw(&raw),
                fields,
                flags,
                raw,
                folders: targets,
            }))
        }
    }
}

/// Lê no fundo (arquivo e MIME) e grava em lotes. Devolve por que parou, ou `None` se terminou.
async fn run(app: &AppHandle, account_id: &str, cancel: Arc<AtomicBool>) -> Option<&'static str> {
    let state = app.state::<AppState>();
    let id = account_id.to_owned();
    let loaded = state.db.call(move |c| Ok((archive_store::get_archive(c, &id)?, archive_store::folder_ids(c, &id)?, store::list_folders(c, &id)?))).await;
    let Ok((Some(archive), folder_ids, folder_rows)) = loaded else { return Some("error") };
    let Some(format) = Format::parse(&archive.format) else { return Some("error") };
    let source = PathBuf::from(&archive.source_path);
    if !source.exists() {
        return Some("source_missing");
    }
    let names: HashMap<i64, String> = folder_rows.into_iter().map(|f| (f.id, f.display_name)).collect();
    let selected: HashSet<String> = archive.selected.iter().cloned().collect();
    // Poucas na fila: cada mensagem pode ter até 50 MB, e quem segura o ritmo é a gravação no banco.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Imported, mail_import::Failure>>(8);

    let reader_cancel = cancel.clone();
    let reader = tauri::async_runtime::spawn_blocking(move || {
        let parser = MessageParser::default();
        mail_import::read(&source, format, &selected, &reader_cancel, |item| {
            let sent = match prepare(&parser, &folder_ids, item) {
                Some(prepared) => tx.blocking_send(prepared),
                None => Ok(()),
            };
            sent.is_ok() && !reader_cancel.load(Ordering::Relaxed)
        })
    });

    let (mut imported, mut duplicates) = (archive.imported, archive.duplicates);
    let mut failures: BTreeMap<String, u32> = archive.failures.clone();
    // Retomar relê o arquivo: as que já entraram contam como repetidas e são descontadas no fim.
    let resumed_from = (imported, duplicates);
    let (mut batch, mut batch_bytes) = (Vec::new(), 0usize);
    let mut stopped = None;
    let data = data_dir(app);
    let mut batches = 0u32;
    loop {
        let next = rx.recv().await;
        let done = next.is_none();
        match next {
            Some(Ok(item)) => {
                batch_bytes += item.raw.len();
                batch.push(item);
            }
            Some(Err(f)) => *failures.entry(f.as_str().to_owned()).or_default() += 1,
            None => {}
        }
        if batch.len() >= BATCH || batch_bytes >= BATCH_BYTES || (done && !batch.is_empty()) {
            let items = std::mem::take(&mut batch);
            batch_bytes = 0;
            let folder = items.last().and_then(|i| i.folders.first()).and_then(|f| names.get(f).cloned());
            let (id, f) = (account_id.to_owned(), failures.clone());
            let written = state
                .db
                .call(move |c| {
                    let r = archive_store::import_batch(c, &id, &items)?;
                    Ok(r)
                })
                .await;
            match written {
                Ok(r) => {
                    imported += r.imported;
                    duplicates += r.duplicates;
                }
                Err(e) => {
                    tracing::warn!(kind = e.kind(), "lote da importação não gravado");
                    stopped = Some("error");
                    cancel.store(true, Ordering::Relaxed);
                    break;
                }
            }
            let id = account_id.to_owned();
            let _ = state.db.call(move |c| archive_store::set_progress(c, &id, imported, duplicates, &f)).await;
            let _ = app.emit(
                "import-progress",
                Progress { account_id: account_id.to_owned(), imported, duplicates, failed: failures.values().sum(), estimate: archive.estimate, folder },
            );
            batches += 1;
            if batches % 10 == 1 {
                state.emit_accounts_changed();
            }
            if let Some(free) = data.as_deref().and_then(free_space) {
                if free < DISK_MARGIN / 2 {
                    stopped = Some("no_space");
                    cancel.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
        if done {
            break;
        }
        if cancel.load(Ordering::Relaxed) && stopped.is_none() {
            stopped = Some("cancelled");
        }
    }
    drop(rx);
    match reader.await {
        Ok(Err(AppError::ImportSourceMissing)) => stopped = stopped.or(Some("source_missing")),
        Ok(Err(_)) | Err(_) => stopped = stopped.or(Some("error")),
        Ok(Ok(())) => {}
    }
    // Ao retomar, o que já tinha entrado volta como "repetida": não é repetida de verdade.
    if resumed_from != (0, 0) {
        let again = duplicates.saturating_sub(resumed_from.1).min(resumed_from.0);
        duplicates -= again;
        let id = account_id.to_owned();
        let _ = state.db.call(move |c| archive_store::set_progress(c, &id, imported, duplicates, &failures)).await;
    }
    stopped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needed_space_never_counts_takeout_labels_twice() {
        let f = |key: &str, bytes| SourceFolder { key: key.into(), name: key.into(), role: crate::mailbox::Role::Custom, depth: 0, count: 1, bytes, label: false, selected: true };
        let scan = Scan { format: Format::Takeout, name: "t".into(), bytes: 100, estimate: 3, folders: vec![f("inbox", 60), f("all", 100), f("label:x", 30)] };
        let all: HashSet<String> = ["inbox", "all", "label:x"].iter().map(|s| s.to_string()).collect();
        assert_eq!(needed_bytes(&scan, &all), 100);
        let some: HashSet<String> = ["label:x"].iter().map(|s| s.to_string()).collect();
        assert_eq!(needed_bytes(&scan, &some), 30);
    }

    /// Do arquivo ao banco: um Takeout lido, preparado e gravado duas vezes (a segunda só repete).
    #[test]
    fn takeout_lands_in_folders_and_labels_without_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Todos.mbox");
        std::fs::write(
            &path,
            concat!(
                "From 1@xxx Mon Jan 1 2024\nX-Gmail-Labels: Inbox,Trabalho\nMessage-ID: <a@x>\nFrom: Ana <ana@x.com>\n",
                "Date: Mon, 1 Jan 2024 10:00:00 +0000\nSubject: Proposta\n\nSegue.\n\n",
                "From 2@xxx Mon Jan 1 2024\nX-Gmail-Labels: Sent,Unread\nMessage-ID: <b@x>\nIn-Reply-To: <a@x>\nFrom: Eu <eu@x.com>\n",
                "Date: Tue, 2 Jan 2024 10:00:00 +0000\nSubject: Re: Proposta\n\nObrigado.\n\n",
                "From 3@xxx Mon Jan 1 2024\nlixo sem cabeçalho nenhum\n",
            ),
        )
        .unwrap();
        let cancel = AtomicBool::new(false);
        let scan = mail_import::scan(&path, &cancel).unwrap();
        let selected: HashSet<String> = scan.folders.iter().filter(|f| f.selected).map(|f| f.key.clone()).collect();
        let t = crate::store::tests::test_db();
        t.db.with(|c| {
            let new = NewArchive { name: "Gmail antigo", format: "takeout", source_name: &scan.name, source_path: "C:/x/Todos.mbox", estimate: scan.estimate };
            let wanted: Vec<SourceFolder> = scan.folders.iter().filter(|f| selected.contains(&f.key)).cloned().collect();
            let account = archive_store::create_archive(c, "11111111-2222-4333-8444-555555555555", &new, &wanted).unwrap();
            let ids = archive_store::folder_ids(c, &account.id).unwrap();
            for round in 0..2 {
                let parser = MessageParser::default();
                let mut items = Vec::new();
                let mut failed = 0;
                mail_import::read(&path, Format::Takeout, &selected, &cancel, |item| {
                    match prepare(&parser, &ids, item) {
                        Some(Ok(i)) => items.push(i),
                        Some(Err(_)) => failed += 1,
                        None => {}
                    }
                    true
                })
                .unwrap();
                let r = archive_store::import_batch(c, &account.id, &items).unwrap();
                if round == 0 {
                    assert_eq!((r.imported, r.duplicates, failed), (2, 0, 1), "a terceira não é uma mensagem: falha como ilegível");
                } else {
                    assert_eq!((r.imported, r.duplicates), (0, 2), "importar de novo não duplica");
                }
            }
            archive_store::order_by_date(c, &account.id).unwrap();
            let folders = crate::store::list_folders(c, &account.id).unwrap();
            let names: Vec<(&str, crate::mailbox::Role, Option<u32>)> = folders.iter().map(|f| (f.display_name.as_str(), f.role, f.total)).collect();
            assert!(names.contains(&("Caixa de entrada", crate::mailbox::Role::Inbox, Some(1))));
            assert!(names.contains(&("Enviados", crate::mailbox::Role::Sent, Some(1))));
            assert!(names.contains(&("Trabalho", crate::mailbox::Role::Custom, Some(1))), "o marcador virou pasta, como numa conta do Google");
            assert!(names.contains(&("Todos os e-mails", crate::mailbox::Role::All, Some(2))));
            let sent = folders.iter().find(|f| f.role == crate::mailbox::Role::Sent).unwrap();
            assert_eq!(sent.unread, Some(1), "o marcador Unread vira não lida");
            let all = folders.iter().find(|f| f.role == crate::mailbox::Role::All).unwrap();
            assert_eq!(crate::store::list_page(c, all.id, None, 10).unwrap().len(), 1, "a resposta e a original numa conversa só");
            Ok(())
        })
        .unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn free_space_is_known_on_windows() {
        assert!(free_space(&std::env::temp_dir()).is_some_and(|b| b > 0));
    }

    #[cfg(unix)]
    #[test]
    fn free_space_is_known_on_unix() {
        assert!(free_space(&std::env::temp_dir()).is_some_and(|b| b > 0));
    }
}
