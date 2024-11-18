use base64::Engine;
use reqwest::header::HeaderValue;

use base64::prelude::BASE64_STANDARD;
use base64::write::EncoderWriter;
use ssh_encoding::Encode;
use ssh_key::HashAlg;
use std::env;
use std::fmt::Debug;
use std::io::Write;
use std::process::Command;
use std::time::SystemTime;

#[async_trait::async_trait]
pub trait AuthMethod: Debug + Sync + Send {
    fn method_name(&self) -> &str;
    fn username(&self) -> &str;
    async fn authenticate(&self, realm: &str) -> HeaderValue;
}

#[async_trait::async_trait]
pub trait IntoPassword: Sync + Send {
    async fn pass(&self) -> String;
}

#[async_trait::async_trait]
impl IntoPassword for String {
    async fn pass(&self) -> String {
        self.to_owned()
    }
}

#[async_trait::async_trait]
impl<F> IntoPassword for F
where
    F: Fn() -> String + Send + Sync,
{
    async fn pass(&self) -> String {
        self()
    }
}

pub struct BasicAuth {
    pub username: String,
    pub password: Box<dyn IntoPassword>,
}

impl Debug for BasicAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicAuth")
            .field("username", &self.username)
            .field("password", &"****")
            .finish()
    }
}

#[async_trait::async_trait]
impl AuthMethod for BasicAuth {
    fn method_name(&self) -> &str {
        "Basic"
    }

    async fn authenticate(&self, _realm: &str) -> HeaderValue {
        basic_auth(&self.username, Some(self.password.pass().await))
    }

    fn username(&self) -> &str {
        &self.username
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

#[async_trait::async_trait]
impl AuthMethod for SSHAuth {
    fn method_name(&self) -> &str {
        "Signature"
    }

    fn username(&self) -> &str {
        &self.username
    }

    async fn authenticate(&self, realm: &str) -> HeaderValue {
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
        let key = match self.ssh_key.is_encrypted() {
            true => {
                let pass = askpass();
                self.ssh_key.decrypt(pass).unwrap()
            }
            false => self.ssh_key.clone(),
        };
        let sig = key
            .sign(realm, HashAlg::default(), data.as_bytes())
            .unwrap();
        let mut writer = Vec::new();
        sig.encode(&mut writer).unwrap();
        base64::engine::general_purpose::STANDARD.encode(writer)
    }

    pub fn new(username: &str, path: &str) -> Result<Self, ssh_key::Error> {
        let path = match path.starts_with(['/', '~']) {
            true => path,
            false => &format!("~/.ssh/{}", path),
        };
        let path = expanduser::expanduser(path).unwrap();
        let key = ssh_key::PrivateKey::read_openssh_file(&path)?;
        Ok(SSHAuth {
            ssh_key: key,
            username: username.to_string(),
        })
    }
}

pub(crate) fn askpass() -> String {
    match env::var_os("SSH_ASKPASS") {
        Some(p) => {
            let mut pass = String::from_utf8(Command::new(p).output().unwrap().stdout).unwrap();
            pass.pop();
            pass
        }
        None => dialoguer::Password::new()
            .with_prompt("Enter password:")
            .interact()
            .unwrap(),
    }
}
