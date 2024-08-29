use std::{fmt::Debug, hash::Hash, sync::Arc};

use reqwest::{
    header::{AUTHORIZATION, WWW_AUTHENTICATE},
    RequestBuilder, Response, StatusCode,
};
use reqwest_cookie_store::CookieStoreRwLock;
use tokio::sync::OnceCell;
use url::Url;

use crate::{authentication, error::APIError};

#[derive(Clone)]
pub struct OBSClient {
    http_client: reqwest::Client,
    api_url: Url,
    authenticator: Arc<dyn authentication::AuthMethod>,
    configuration: Arc<OnceCell<Configuration>>,
}

impl PartialEq for OBSClient {
    fn eq(&self, other: &Self) -> bool {
        self.api_url == other.api_url
    }
}

impl Eq for OBSClient {}

impl Hash for OBSClient {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.api_url.hash(state);
    }
}

impl Debug for OBSClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OBSClient")
            .field("api_url", &self.api_url.as_str())
            .field("authenticator", &self.authenticator)
            .field("configuration", &self.configuration)
            .finish_non_exhaustive()
    }
}
#[derive(Debug, yaserde::YaDeserialize)]
struct Configuration {
    obs_url: String,
}

impl OBSClient {
    pub fn new(
        api_url: Url,
        authenticator: Arc<dyn authentication::AuthMethod>,
        cookie_jar: Option<Arc<CookieStoreRwLock>>,
    ) -> Result<Self, APIError> {
        //TODO: Add cookie Jar
        let mut http_client_builder = reqwest::Client::builder();
        if let Some(cookie_store) = cookie_jar {
            http_client_builder = http_client_builder.cookie_provider(cookie_store);
        }
        let http_client = http_client_builder.build()?;
        Ok(OBSClient {
            http_client,
            api_url,
            authenticator,
            configuration: Default::default(),
        })
    }

    pub(crate) fn get(&self, route: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();
        url.path_segments_mut().expect("Base url").extend(route);
        self.http_client.get(url)
    }

    pub(crate) fn post(&self, route: &[&str]) -> RequestBuilder {
        let mut url = self.api_url.clone();
        url.path_segments_mut().expect("Base url").extend(route);
        self.http_client.post(url)
    }

    pub(crate) async fn execute(
        &self,
        request: reqwest::Request,
    ) -> Result<Response, reqwest::Error> {
        let req_bak = request.try_clone();
        let resp = self.http_client.execute(request).await?;
        if resp.status().is_success() {
            return Ok(resp);
        }

        if resp.status() == StatusCode::UNAUTHORIZED {
            if let Some(mut req_bak) = req_bak {
                let auth = resp.headers().get_all(WWW_AUTHENTICATE);
                for auth_method in auth {
                    if let Some(auth_params) = auth_method
                        .to_str()
                        .unwrap_or_default()
                        .strip_prefix(self.authenticator.method_name())
                    {
                        if let Some(realm) = auth_params
                            .trim()
                            .split(',')
                            .filter(|s| !s.is_empty())
                            .find_map(|s| Some(s.strip_prefix("realm=")?.trim_matches('"')))
                        {
                            req_bak
                                .headers_mut()
                                .insert(AUTHORIZATION, self.authenticator.authenticate(realm));
                            return self.http_client.execute(req_bak).await?.error_for_status();
                        }
                    }
                }
            }
        }

        resp.error_for_status()
    }

    async fn init_configuration(&self) -> Configuration {
        let resp = self
            .execute(
                self.get(&["configuration"])
                    .header("Accept", "application/xml; charset=utf-8")
                    .build()
                    .unwrap(),
            )
            .await
            .unwrap();
        yaserde::de::from_str(&resp.text().await.unwrap()).unwrap()
    }

    pub async fn get_obs_route(&self, route: &[&str]) -> Url {
        let mut url = Url::parse(
            &self
                .configuration
                .get_or_init(|| async { self.init_configuration().await })
                .await
                .obs_url,
        )
        .unwrap();
        url.path_segments_mut().unwrap().extend(route);
        url
    }
}
