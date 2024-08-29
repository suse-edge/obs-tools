use base64::Engine;
use reqwest::header::HeaderValue;

use base64::prelude::BASE64_STANDARD;
use base64::write::EncoderWriter;
use ssh_key::HashAlg;
use std::fmt::Debug;
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

pub trait AuthMethod: Debug + Sync + Send {
    fn method_name(&self) -> &str;
    fn authenticate(&self, realm: &str) -> HeaderValue;
}

pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

impl Debug for BasicAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicAuth")
            .field("username", &self.username)
            .field("password", &"****")
            .finish()
    }
}

impl AuthMethod for BasicAuth {
    fn method_name(&self) -> &str {
        "Basic"
    }

    fn authenticate(&self, _realm: &str) -> HeaderValue {
        basic_auth(&self.username, Some(&self.password))
    }
}

/// Taken verbatim from reqwest::util
fn basic_auth<U, P>(username: U, password: Option<P>) -> HeaderValue
where
    U: std::fmt::Display,
    P: std::fmt::Display,
{
    let mut buf = b"Basic ".to_vec();
    {
        let mut encoder = EncoderWriter::new(&mut buf, &BASE64_STANDARD);
        let _ = write!(encoder, "{username}:");
        if let Some(password) = password {
            let _ = write!(encoder, "{password}");
        }
    }
    let mut header = HeaderValue::from_bytes(&buf).expect("base64 is always valid HeaderValue");
    header.set_sensitive(true);
    header
}

#[derive(Debug)]
pub struct SSHAuth {
    pub ssh_key: ssh_key::PrivateKey,
    pub username: String,
}

impl AuthMethod for SSHAuth {
    fn method_name(&self) -> &str {
        "Signature"
    }

    fn authenticate(&self, realm: &str) -> HeaderValue {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Before epoch")
            .as_secs();
        let sig = self.get_signature(realm, &format!("(created): {}", now));
        let buf = format!(
            r#"Signature keyId="{}",algorithm="ssh",headers="(created)",created={},signature="{}""#,
            self.username, now, sig,
        )
        .into_bytes();
        let mut header = HeaderValue::from_bytes(&buf).expect("should always be valid HeaderValue");
        header.set_sensitive(true);
        header
    }
}

impl SSHAuth {
    fn get_signature(&self, realm: &str, data: &str) -> String {
        let sig = self
            .ssh_key
            .sign(realm, HashAlg::default(), data.as_bytes())
            .unwrap();
        base64::engine::general_purpose::STANDARD.encode(sig.signature_bytes())
    }

    pub fn new(username: &str, path: &Path) -> Result<Self, ssh_key::Error> {
        let key = ssh_key::PrivateKey::read_openssh_file(path)?;
        Ok(SSHAuth {
            ssh_key: key,
            username: username.to_string(),
        })
    }
}
