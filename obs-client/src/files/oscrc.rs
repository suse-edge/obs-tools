use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use configparser::ini::{Ini, IniDefault};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use url::Url;

use crate::authentication::IntoPassword;

const GENERAL_SECTION: &str = "general";

#[non_exhaustive]
pub struct Oscrc {
    pub hosts_options: HashMap<Url, HostOptions>,

    pub apiurl: Url,
    pub sshkey: Option<String>,
    pub http_retries: u64,
    pub cookiejar: PathBuf,
    pub realname: Option<String>,
    pub email: Option<String>,
}

pub struct HostOptions {
    pub aliases: Vec<String>,
    pub username: String,
    pub credential_class: CredentialsManagers,
    pub password: Option<String>,
    pub sshkey: Option<String>,
    pub cafile: Option<PathBuf>,
    pub capath: Option<PathBuf>,
    pub http_headers: HeaderMap,
    pub realname: Option<String>,
    pub email: Option<String>,
}

impl Default for Oscrc {
    fn default() -> Self {
        let bd = xdg::BaseDirectories::with_prefix("osc").unwrap();
        Self {
            hosts_options: Default::default(),

            apiurl: Url::parse("https://api.opensuse.org").unwrap(),
            sshkey: None,
            http_retries: 3,
            cookiejar: bd.get_state_file("cookiejar"),
            realname: None,
            email: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    #[error("XDG Directory error")]
    XDGError(#[from] xdg::BaseDirectoriesError),
    #[error("Error on field:")]
    FieldError {
        section: String,
        field: &'static str,
        error: String,
    },
    #[error("Error parsing url")]
    URLParseError(#[from] url::ParseError),
    #[error("Unable to read file")]
    FileError(String),
}

impl Oscrc {
    pub fn new(path_override: Option<&Path>) -> Result<Self, ParseError> {
        let cfgfiles = match path_override {
            Some(file) => vec![file.to_path_buf()],
            None => {
                let bd = xdg::BaseDirectories::with_prefix("osc")?;
                bd.find_config_files("oscrc").collect()
            }
        };
        let mut defaults = IniDefault::default();
        defaults.multiline = true;
        defaults.delimiters = vec!['='];
        let mut cfg = Ini::new_from_defaults(defaults);
        for file in cfgfiles {
            cfg.load_and_append(file).map_err(ParseError::FileError)?;
        }

        let mut config = Self::default();
        if let Some(apiurl) = cfg.get(GENERAL_SECTION, "apiurl") {
            let apiurl = Url::parse(&apiurl)?;
            config.apiurl = apiurl;
        }
        config.sshkey = cfg.get(GENERAL_SECTION, "sshkey");
        if let Some(http_retries) =
            cfg.getuint(GENERAL_SECTION, "http_retries")
                .map_err(|e| ParseError::FieldError {
                    section: GENERAL_SECTION.to_string(),
                    field: "http_retries",
                    error: e,
                })?
        {
            config.http_retries = http_retries
        }
        if let Some(cookiejar) = cfg.get(GENERAL_SECTION, "cookiejar") {
            config.cookiejar = cookiejar.into();
        }
        config.realname = cfg.get(GENERAL_SECTION, "realname");
        config.email = cfg.get(GENERAL_SECTION, "email");
        cfg.remove_section(GENERAL_SECTION);
        for section in cfg.sections() {
            let url = Url::parse(&section)?;
            let hopts = HostOptions {
                aliases: vec![],
                username: cfg.get(&section, "user").ok_or(ParseError::FieldError {
                    section: section.clone(),
                    field: "user",
                    error: "Not found".to_string(),
                })?,
                credential_class: cfg
                    .get(&section, "credentials_mgr_class")
                    .unwrap_or_default()
                    .into(),
                password: cfg.get(&section, "password"),
                sshkey: cfg.get(&section, "sshkey"),
                cafile: cfg.get(&section, "cafile").map(PathBuf::from),
                capath: cfg.get(&section, "capath").map(PathBuf::from),
                realname: cfg.get(&section, "realname"),
                email: cfg.get(&section, "email"),
                http_headers: parse_headers(&cfg.get(&section, "http_headers").unwrap_or_default()),
            };
            config.hosts_options.insert(url, hopts);
        }

        Ok(config)
    }

    pub fn apiurl_from_alias(&self, alias: &str) -> Option<Url> {
        match Url::parse(alias) {
            Ok(u) => match self.hosts_options.contains_key(&u) {
                true => Some(u),
                false => None,
            },
            Err(_) => self.hosts_options.iter().find_map(|(u, opt)| {
                match opt.aliases.contains(&alias.to_string()) {
                    true => Some(u.clone()),
                    false => None,
                }
            }),
        }
    }

    pub fn get_password_provider(&self, api_url: &Url) -> Box<dyn IntoPassword> {
        let host_options = &self.hosts_options[api_url];
        match host_options.credential_class {
            CredentialsManagers::Plaintext => {
                Box::new(host_options.password.clone().unwrap_or_default())
            }
            CredentialsManagers::Transient => Box::new(crate::authentication::askpass),
            CredentialsManagers::KernelKeyring => {
                tracing::warn!(
                    "Kernel Keyring backend not implemented yet, falling back to Transient"
                );
                Box::new(crate::authentication::askpass)
            }
            CredentialsManagers::SecretService => {
                tracing::warn!(
                    "Secret Service backend not implemented yet, falling back to Transient"
                );
                Box::new(crate::authentication::askpass)
            }
            CredentialsManagers::Kwallet => Box::new(crate::kwallet::KWalletGetter::new(
                api_url,
                &self.hosts_options[api_url].username,
            )),
        }
    }
}

#[non_exhaustive]
pub enum CredentialsManagers {
    Transient,
    Plaintext,
    KernelKeyring,
    SecretService,
    Kwallet,
}

impl Default for CredentialsManagers {
    fn default() -> Self {
        Self::Plaintext
    }
}

impl From<String> for CredentialsManagers {
    fn from(value: String) -> Self {
        match value.as_str() {
            "osc.credentials.TransientCredentialsManager" => Self::Transient,
            "osc.credentials.KeyringCredentialsManager:keyring.backends.kwallet.DBusKeyring" => {
                Self::Kwallet
            }
            "osc.credentials.KeyringCredentialsManager:keyutils.osc.OscKernelKeyringBackend" => {
                Self::KernelKeyring
            }
            "osc.credentials.KeyringCredentialsManager:keyring.backends.SecretService.Keyring" => {
                Self::SecretService
            }
            _ => Self::Plaintext,
        }
    }
}

fn parse_headers(input: &str) -> HeaderMap {
    input
        .lines()
        .filter_map(|line| line.split_once(':'))
        .filter_map(|(k, v)| {
            let k = HeaderName::from_str(k).ok()?;
            let v = HeaderValue::from_str(v).ok()?;
            Some((k, v))
        })
        .collect()
}
