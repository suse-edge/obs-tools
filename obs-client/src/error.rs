use thiserror::Error;

#[derive(Debug, Error)]
pub enum APIError {
    #[error("Unable to deserialize XML")]
    XMLParseError(String),
    #[error("HTTP Error")]
    HTTPError(#[from] reqwest::Error),
    #[error("Unable to parse CookieJar file")]
    CookieError(#[from] cookie_store::CookieError),
}
