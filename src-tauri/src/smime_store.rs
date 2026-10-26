//! Certificados S/MIME e pedidos no banco (US14). O banco é cifrado (US3, US10); a chave privada dos
//! meus certificados fica sem senha própria aqui dentro. A confiança é calculada na hora, pelas raízes.

use openssl::x509::X509;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::db::sql_error;
use crate::error::AppError;
use crate::smime::{self, Trust};
use crate::store::now;

/// Um certificado como o gerenciador o mostra (CA2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmimeCertRow {
    pub fingerprint: String,
    pub subject: String,
    pub organization: Option<String>,
    pub emails: Vec<String>,
    pub issuer: String,
    pub serial: String,
    pub algorithm: String,
    pub not_before: i64,
    pub not_after: i64,
    pub can_sign: bool,
    pub can_encrypt: bool,
    pub self_signed: bool,
    pub origin: String,
    pub added_at: i64,
    /// Tem a chave privada (é meu).
    pub mine: bool,
    pub trust: Trust,
    /// Titulares da cadeia, do emissor até a raiz que veio junto.
    pub chain: Vec<String>,
}

/// O certificado guardado, como o OpenSSL o usa.
pub struct Stored {
    pub cert: X509,
    pub chain: Vec<X509>,
    pub private_key: Option<zeroize::Zeroizing<Vec<u8>>>,
    pub origin: String,
    pub added_at: i64,
}

impl Stored {
    pub fn row(&self, roots: &[X509]) -> SmimeCertRow {
        let info = smime::info(&self.cert);
        SmimeCertRow {
            trust: smime::trust(&self.cert, &self.chain, roots, false),
            chain: self.chain.iter().map(|c| smime::info(c).subject).collect(),
            mine: self.private_key.is_some(),
            fingerprint: info.fingerprint,
            subject: info.subject,
            organization: info.organization,
            emails: info.emails,
            issuer: info.issuer,
            serial: info.serial,
            algorithm: info.algorithm,
            not_before: info.not_before,
            not_after: info.not_after,
            can_sign: info.can_sign,
            can_encrypt: info.can_encrypt,
            self_signed: info.self_signed,
            origin: self.origin.clone(),
            added_at: self.added_at,
        }
    }
}

const SELECT: &str = "SELECT der, chain_pem, private_key, origin, added_at FROM smime_cert";

/// Uma linha de `smime_cert` antes de o OpenSSL ler: DER, cadeia, chave, origem, quando entrou.
type RawCert = (Vec<u8>, String, Option<Vec<u8>>, String, i64);

/// O pedido que casa com um certificado: id, e-mail pedido e a chave privada.
pub type PendingKey = (i64, String, zeroize::Zeroizing<Vec<u8>>);

fn stored(r: &rusqlite::Row<'_>) -> rusqlite::Result<RawCert> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
}

fn parse((der, chain, key, origin, added_at): RawCert) -> Option<Stored> {
    Some(Stored { cert: X509::from_der(&der).ok()?, chain: smime::chain_from_pem(&chain), private_key: key.map(zeroize::Zeroizing::new), origin, added_at })
}

/// Grava um certificado novo ou atualiza um conhecido. A chave privada só é acrescentada; a cadeia
/// só é trocada por uma que não esteja vazia.
pub fn upsert_cert(conn: &mut Connection, cert: &X509, chain: &[X509], private_key: Option<&[u8]>, origin: &str) -> Result<Stored, AppError> {
    let info = smime::info(cert);
    let der = smime::cert_der(cert)?;
    let tx = conn.transaction().map_err(sql_error("certificados"))?;
    tx.execute(
        "INSERT INTO smime_cert (fingerprint, der, chain_pem, private_key, subject, organization, issuer, serial, algorithm, not_before, not_after, can_sign, can_encrypt, self_signed, origin, added_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
         ON CONFLICT (fingerprint) DO UPDATE SET private_key = coalesce(excluded.private_key, smime_cert.private_key),
            chain_pem = CASE WHEN excluded.chain_pem <> '' THEN excluded.chain_pem ELSE smime_cert.chain_pem END",
        params![
            info.fingerprint,
            der,
            smime::chain_pem(chain),
            private_key,
            info.subject,
            info.organization,
            info.issuer,
            info.serial,
            info.algorithm,
            info.not_before,
            info.not_after,
            info.can_sign,
            info.can_encrypt,
            info.self_signed,
            origin,
            now(),
        ],
    )
    .map_err(sql_error("certificados"))?;
    for email in &info.emails {
        tx.execute("INSERT OR IGNORE INTO smime_cert_email (email, fingerprint) VALUES (?1, ?2)", params![email, info.fingerprint]).map_err(sql_error("certificados"))?;
    }
    tx.commit().map_err(sql_error("certificados"))?;
    get_cert(conn, &info.fingerprint)?.ok_or(AppError::Storage)
}

pub fn get_cert(conn: &Connection, fingerprint: &str) -> Result<Option<Stored>, AppError> {
    let raw = conn.query_row(&format!("{SELECT} WHERE fingerprint = ?1"), [fingerprint], stored).optional().map_err(sql_error("certificados"))?;
    Ok(raw.and_then(parse))
}

pub fn list_certs(conn: &Connection) -> Result<Vec<Stored>, AppError> {
    let mut st = conn.prepare_cached(&format!("{SELECT} ORDER BY private_key IS NULL, lower(subject)")).map_err(sql_error("certificados"))?;
    let rows = st.query_map([], stored).map_err(sql_error("certificados"))?;
    let raw: Vec<_> = rows.collect::<Result<_, _>>().map_err(sql_error("certificados"))?;
    Ok(raw.into_iter().filter_map(parse).collect())
}

pub fn delete_cert(conn: &Connection, fingerprint: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM smime_cert WHERE fingerprint = ?1", [fingerprint]).map_err(sql_error("certificados"))?;
    Ok(())
}

fn certs_of(conn: &Connection, email: &str) -> Result<Vec<Stored>, AppError> {
    let mut st = conn
        .prepare_cached("SELECT c.der, c.chain_pem, c.private_key, c.origin, c.added_at FROM smime_cert c JOIN smime_cert_email e ON e.fingerprint = c.fingerprint WHERE e.email = ?1 ORDER BY c.not_after DESC")
        .map_err(sql_error("certificados"))?;
    let rows = st.query_map([email.trim().to_lowercase()], stored).map_err(sql_error("certificados"))?;
    let raw: Vec<_> = rows.collect::<Result<_, _>>().map_err(sql_error("certificados"))?;
    Ok(raw.into_iter().filter_map(parse).collect())
}

/// Certificados de um e-mail, quaisquer (para verificar assinaturas).
pub fn certs_for(conn: &Connection, email: &str) -> Result<Vec<X509>, AppError> {
    Ok(certs_of(conn, email)?.into_iter().map(|s| s.cert).collect())
}

/// Certificado de um e-mail que serve para cifrar: uso de cifra, dentro da validade e com a cadeia
/// confiável (o meu vale mesmo sem a cadeia). Entre vários, o que vence por último.
pub fn usable_cert_for(conn: &Connection, email: &str, roots: &[X509]) -> Result<Option<Stored>, AppError> {
    let at = now();
    Ok(certs_of(conn, email)?.into_iter().find(|s| {
        let info = smime::info(&s.cert);
        info.can_encrypt && smime::current(&info, at) && (s.private_key.is_some() || smime::trust(&s.cert, &s.chain, roots, true) == Trust::Trusted)
    }))
}

/// Meus certificados com chave, para decifrar o que chegar para qualquer um deles.
pub fn my_certs(conn: &Connection) -> Result<Vec<Stored>, AppError> {
    Ok(list_certs(conn)?.into_iter().filter(|s| s.private_key.is_some()).collect())
}

// ---------------------------------------------------------------- pedidos (CA1)

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmimeRequestRow {
    pub id: i64,
    pub account_id: String,
    pub email: String,
    pub name: String,
    pub organization: Option<String>,
    pub country: Option<String>,
    pub algorithm: String,
    pub csr_pem: String,
    pub created_at: i64,
}

pub struct NewRequest<'a> {
    pub account_id: &'a str,
    pub email: &'a str,
    pub name: &'a str,
    pub organization: Option<&'a str>,
    pub country: Option<&'a str>,
    pub algorithm: &'a str,
}

const REQUEST_SELECT: &str = "SELECT id, account_id, email, name, organization, country, algorithm, csr_pem, created_at FROM smime_request";

fn request_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<SmimeRequestRow> {
    Ok(SmimeRequestRow {
        id: r.get(0)?,
        account_id: r.get(1)?,
        email: r.get(2)?,
        name: r.get(3)?,
        organization: r.get(4)?,
        country: r.get(5)?,
        algorithm: r.get(6)?,
        csr_pem: r.get(7)?,
        created_at: r.get(8)?,
    })
}

pub fn insert_request(conn: &Connection, new: &NewRequest<'_>, request: &smime::Request) -> Result<SmimeRequestRow, AppError> {
    conn.execute(
        "INSERT INTO smime_request (account_id, email, name, organization, country, algorithm, private_key, public_key, csr_pem, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![new.account_id, new.email, new.name, new.organization, new.country, new.algorithm, request.private_key.as_slice(), request.public_key, request.csr_pem, now()],
    )
    .map_err(sql_error("pedidos"))?;
    let id = conn.last_insert_rowid();
    conn.query_row(&format!("{REQUEST_SELECT} WHERE id = ?1"), [id], request_row).map_err(sql_error("pedidos"))
}

pub fn list_requests(conn: &Connection) -> Result<Vec<SmimeRequestRow>, AppError> {
    let mut st = conn.prepare_cached(&format!("{REQUEST_SELECT} ORDER BY created_at DESC")).map_err(sql_error("pedidos"))?;
    let rows = st.query_map([], request_row).map_err(sql_error("pedidos"))?;
    rows.collect::<Result<_, _>>().map_err(sql_error("pedidos"))
}

/// O pedido cuja chave pública é a do certificado emitido: o id, o e-mail pedido e a chave privada.
pub fn request_for_key(conn: &Connection, public_key: &[u8]) -> Result<Option<PendingKey>, AppError> {
    conn.query_row("SELECT id, email, private_key FROM smime_request WHERE public_key = ?1", [public_key], |r| {
        Ok((r.get(0)?, r.get(1)?, zeroize::Zeroizing::new(r.get::<_, Vec<u8>>(2)?)))
    })
    .optional()
    .map_err(sql_error("pedidos"))
}

pub fn delete_request(conn: &Connection, id: i64) -> Result<(), AppError> {
    conn.execute("DELETE FROM smime_request WHERE id = ?1", [id]).map_err(sql_error("pedidos"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smime::tests::{issue, person, test_ca};
    use crate::smime::{CertKind, Subject};
    use crate::store::tests::{account, test_db};

    #[test]
    fn certificates_are_found_by_email_and_trust() {
        let t = test_db();
        let (ca, ca_key) = test_ca();
        let roots = std::slice::from_ref(&ca);
        t.db.with(|c| {
            let (ana, _) = person("ana@x.com", CertKind::P256, &ca, &ca_key);
            let row = upsert_cert(c, &ana, &[], None, "message").unwrap().row(roots);
            assert_eq!((row.trust, row.mine), (Trust::Trusted, false));
            assert!(usable_cert_for(c, "ANA@x.com", roots).unwrap().is_some());
            assert!(usable_cert_for(c, "ana@x.com", &[]).unwrap().is_none(), "sem a raiz, não cifra para ela");
            assert_eq!(certs_for(c, "ana@x.com").unwrap().len(), 1, "mas verifica a assinatura");

            let (me, key) = person("eu@x.com", CertKind::Rsa3072, &ca, &ca_key);
            let der = crate::smime::private_key_der(&key).unwrap();
            upsert_cert(c, &me, std::slice::from_ref(&ca), Some(&der), "pkcs12").unwrap();
            // Importar de novo só o certificado não apaga a chave nem a cadeia.
            let again = upsert_cert(c, &me, &[], None, "imported").unwrap();
            assert!(again.private_key.is_some());
            assert_eq!(again.row(roots).chain, vec!["CA de teste do Epistula".to_owned()]);
            assert_eq!(my_certs(c).unwrap().len(), 1);
            assert!(usable_cert_for(c, "eu@x.com", &[]).unwrap().is_some(), "o meu vale sem a cadeia");
            assert_eq!(list_certs(c).unwrap().len(), 2);
            delete_cert(c, &crate::smime::fingerprint(&ana)).unwrap();
            assert!(certs_for(c, "ana@x.com").unwrap().is_empty());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn requests_are_matched_by_public_key() {
        let t = test_db();
        let (ca, ca_key) = test_ca();
        t.db.with(|c| {
            let a = account(c, "eu@x.com");
            let req = crate::smime::generate_request(CertKind::P256, &Subject { name: "Eu", email: "eu@x.com", organization: None, country: None }).unwrap();
            let row = insert_request(c, &NewRequest { account_id: &a.id, email: "eu@x.com", name: "Eu", organization: None, country: None, algorithm: "ECDSA P-256" }, &req).unwrap();
            assert_eq!(list_requests(c).unwrap().len(), 1);
            let cert = issue(&row.csr_pem, &ca, &ca_key, 0, 365);
            let (id, email, key) = request_for_key(c, &crate::smime::public_key_der(&cert).unwrap()).unwrap().unwrap();
            assert_eq!((id, email.as_str()), (row.id, "eu@x.com"));
            assert_eq!(key.as_slice(), req.private_key.as_slice());
            delete_request(c, id).unwrap();
            assert!(list_requests(c).unwrap().is_empty());
            Ok(())
        })
        .unwrap();
    }
}
