use std::{io::Read, path::PathBuf, sync::Arc};

use reqwest::Url;
use serde::Deserialize;
use tracing::info;

use obs_client::{authentication::BasicAuth, client::OBSClient};

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ObsCredentials {
    Secret(String),
    Plain { login: String, password: String },
}

pub async fn new_client(uri: Url, creds: ObsCredentials) -> anyhow::Result<Arc<OBSClient>> {
    let authenticator: Arc<dyn obs_client::authentication::AuthMethod> = match creds {
        ObsCredentials::Plain { login, password } => Arc::new(BasicAuth {
            username: login,
            password,
        }),
        ObsCredentials::Secret(path) => {
            let login_path: PathBuf = [&path, "username"].iter().collect();
            let mut login = String::default();
            std::fs::File::open(&login_path)?.read_to_string(&mut login)?;
            let sshkey_path: PathBuf = [&path, "ssh-privatekey"].iter().collect();
            if sshkey_path.is_file() {
                Arc::new(obs_client::authentication::SSHAuth::new(
                    &login,
                    &sshkey_path,
                )?)
            } else {
                let password_path: PathBuf = [&path, "password"].iter().collect();
                let mut password = String::default();
                std::fs::File::open(&password_path)?.read_to_string(&mut password)?;
                Arc::new(BasicAuth {
                    username: login,
                    password,
                })
            }
        }
    };
    let client = OBSClient::new(uri, authenticator, None)?;
    let obs_url = client.get_obs_route(&[]).await;
    info!(%obs_url, "Connected to OBS");
    Ok(Arc::new(client))
}
