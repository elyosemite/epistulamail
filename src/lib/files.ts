// Arquivos do usuário sem plugin: salvar vai para a pasta Downloads pelo WebView; abrir usa o seletor do sistema.

/** Salva um texto (chave pública, backup) como arquivo. */
export function saveText(fileName: string, text: string, type = "application/pgp-keys") {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** Lê um arquivo de chave ou certificado: texto em armadura (PGP ou PEM) como está, binário em base64. */
export async function readKeyFile(file: File): Promise<{ data: string; binary: boolean }> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const head = new TextDecoder().decode(bytes.subarray(0, 64)).trimStart();
  if (head.startsWith("-----BEGIN ")) return { data: new TextDecoder().decode(bytes), binary: false };
  let raw = "";
  for (let i = 0; i < bytes.length; i += 0x8000) raw += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return { data: btoa(raw), binary: true };
}

/** Abre o seletor de arquivos e devolve o escolhido (ou null). */
export function pickFile(accept: string): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.onchange = () => resolve(input.files?.[0] ?? null);
    input.oncancel = () => resolve(null);
    input.click();
  });
}

/** Nome de arquivo seguro a partir de um nome de chave. */
export function keyFileName(name: string, suffix: string, extension = "asc"): string {
  const base = name.normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/[^A-Za-z0-9@._-]+/g, "-").replace(/^-+|-+$/g, "") || "chave";
  return `${base}-${suffix}.${extension}`;
}

/** Salva um arquivo binário que veio em base64 (o `.p12` exportado). */
export function saveBase64(fileName: string, data: string, type = "application/x-pkcs12") {
  const raw = atob(data);
  const bytes = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
  const url = URL.createObjectURL(new Blob([bytes], { type }));
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
