use std::{collections::HashMap, sync::Arc};

use amqp::create_client;
use dispatcher::Dispatcher;
use gangway::prow_client;
use prow::ProwConfig;
use serde::Deserialize;
use tokio::{sync::Mutex, try_join};
use tonic::metadata::{Ascii, MetadataValue};
use tracing::debug;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod amqp;
mod obs;

mod dispatcher;
mod gangway;
mod prow;
mod web;

use obs::{new_client, ObsCredentials};
use url::Url;

#[derive(Debug, Clone, Deserialize)]
struct Configuration {
    amqp_uri: String,
    obs_uri: Url,
    obs_credentials: ObsCredentials,
    prow: Option<ProwConfig>,
    projects: Vec<Project>,
}

#[derive(Debug, Clone, Deserialize)]
struct Project {
    name: String,
    #[serde(default)]
    actions: Vec<Action>,
    checks: Vec<Check>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Action {
    ObsRelease,
    ProwJob { name: String, job_type: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Check {
    ProwJob { name: String },
}

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    let path = std::env::var("CONFIG_FILE").unwrap_or("config.yaml".to_string());
    let config_file = std::fs::File::open(&path)
        .unwrap_or_else(|_| panic!("Unable to read the configuration file: {}", &path));
    let config: Configuration =
        serde_yaml::from_reader(config_file).expect("Unable to parse config file");
    debug!(config=?config);

    let projects: HashMap<String, Project> = config
        .projects
        .iter()
        .map(|p| (p.name.clone(), p.clone()))
        .collect();

    let obs_client = new_client(config.obs_uri, config.obs_credentials)
        .await
        .expect("Unable to create OBS client");

    let prow_client = match config.prow {
        None => None,
        Some(prow_config) => {
            let mut consumer_type: MetadataValue<Ascii> = prow_config
                .consumer_type
                .clone()
                .parse()
                .expect("Invalid consumer type metadata");
            consumer_type.set_sensitive(true);
            let mut consumer_number: MetadataValue<Ascii> = prow_config
                .consumer_number
                .clone()
                .parse()
                .expect("Invalid consumer number");
            consumer_number.set_sensitive(true);
            let uri = prow_config.uri.parse().expect("Invalid URI");
            Some(Arc::new(Mutex::new(
                prow_client::ProwClient::with_interceptor(
                    tonic::transport::Channel::builder(uri)
                        .connect()
                        .await
                        .expect("Unable to connect to prow API"),
                    move |mut req: tonic::Request<()>| {
                        req.metadata_mut()
                            .insert("x-endpoint-api-consumer-type", consumer_type.clone());
                        req.metadata_mut()
                            .insert("x-endpoint-api-consumer-number", consumer_number.clone());
                        Ok(req)
                    },
                ),
            )))
        }
    };

    let dispatcher = Arc::new(Dispatcher::new(projects, obs_client.clone(), prow_client));

    let web_app = tokio::spawn(web::run_web_app(
        obs_client.clone(),
        dispatcher.clone(),
        dispatcher.get_projects_list(),
    ));

    let amqp_client = create_client(
        &config.amqp_uri,
        dispatcher.get_projects_list(),
        dispatcher,
        obs_client,
    )
    .await
    .unwrap();

    let _ = try_join!(web_app, amqp_client);
}
