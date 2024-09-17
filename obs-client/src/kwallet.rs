use tokio::sync::OnceCell;
use url::Url;
use zbus::{proxy, Connection, Result};

use crate::authentication::IntoPassword;

#[proxy(
    interface = "org.kde.KWallet",
    default_service = "org.kde.kwalletd5",
    default_path = "/modules/kwalletd5"
)]
trait KWallet {
    #[zbus(name = "networkWallet")]
    async fn network_wallet(&self) -> Result<String>;
    #[zbus(name = "open")]
    async fn open(&self, wallet: &str, w_id: i64, appid: &str) -> Result<i32>;
    #[zbus(name = "hasEntry")]
    async fn has_entry(&self, handle: i32, folder: &str, key: &str, appid: &str) -> Result<bool>;
    #[zbus(name = "readPassword")]
    async fn read_password(
        &self,
        handle: i32,
        folder: &str,
        key: &str,
        appid: &str,
    ) -> Result<String>;
}

const APP_ID: &str = "osc-rs";

#[derive(Debug, Default)]
pub struct KWalletGetter {
    folder: String,
    key: String,
    handle: OnceCell<i32>,
    connection: OnceCell<Connection>,
}

impl KWalletGetter {
    pub fn new(api_url: &Url, username: &str) -> Self {
        Self {
            folder: api_url.host_str().unwrap().to_string(),
            key: username.to_string(),
            ..Default::default()
        }
    }
}

#[async_trait::async_trait]
impl IntoPassword for KWalletGetter {
    async fn pass(&self) -> String {
        let connection = self.connection.get_or_init(get_connection).await;
        let proxy = KWalletProxy::new(connection).await.unwrap();
        let handle = self
            .handle
            .get_or_init(|| async {
                let wallet = proxy.network_wallet().await.unwrap();
                proxy.open(&wallet, 0, APP_ID).await.unwrap()
            })
            .await;
        if !proxy
            .has_entry(*handle, &self.folder, &self.key, APP_ID)
            .await
            .unwrap()
        {
            panic!("No password found: {}/{}", &self.folder, &self.key);
        }
        proxy
            .read_password(*handle, &self.folder, &self.key, APP_ID)
            .await
            .unwrap()
    }
}

async fn get_connection() -> Connection {
    Connection::session().await.unwrap()
}
