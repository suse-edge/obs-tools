use std::sync::Arc;

use clap::Parser;
use itertools::Itertools;
use obs_client::{
    api::{project::Project, request::Request},
    files::Oscrc,
};
use package_solver::get_actions;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use url::Url;

mod package_solver;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short, long)]
    api_url: Option<Url>,
    #[arg(short, long)]
    username: Option<String>,
    #[arg(short, long)]
    password: Option<String>,
    #[arg(long)]
    dry_run: bool,
    src_project: String,
    dst_project: String,
    #[arg(long, short)]
    deps_project: Vec<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Cli::parse();

    let cfg = Oscrc::new(None).unwrap();
    let jar = obs_client::get_osc_cookiejar(&cfg).unwrap();
    let api_url = args.api_url.unwrap_or(cfg.apiurl.clone());
    let username = args
        .username
        .unwrap_or(cfg.hosts_options[&api_url].username.clone());

    let auhtenticator: Arc<dyn obs_client::authentication::AuthMethod> = match &cfg.hosts_options
        [&api_url]
        .sshkey
    {
        Some(key) => Arc::new(obs_client::authentication::SSHAuth::new(&username, key).unwrap()),
        None => {
            let auth: Arc<dyn obs_client::authentication::AuthMethod> = match &cfg.sshkey {
                Some(key) => {
                    Arc::new(obs_client::authentication::SSHAuth::new(&username, key).unwrap())
                }
                None => Arc::new(obs_client::authentication::BasicAuth {
                    username,
                    password: match args.password {
                        Some(pass) => Box::new(pass),
                        None => cfg.get_password_provider(&api_url),
                    },
                }),
            };
            auth
        }
    };

    let client = Arc::new(
        obs_client::client::OBSClient::new(api_url.clone(), auhtenticator, Some(jar)).unwrap(),
    );
    let src_project = Project::from_name(client.clone(), &args.src_project);
    let dst_project = Project::from_name(client.clone(), &args.dst_project);
    let src_packages = src_project.packagelist(false).await.unwrap();
    info!("Got src");
    let dst_packages = dst_project.packagelist(false).await.unwrap();
    let actions = get_actions(dst_packages, src_packages, &dst_project, args.deps_project).await;
    if actions.is_empty() {
        info!("Nothing to do exiting");
        return;
    }

    let requests = Request::mine(client.clone())
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.is_for_project(&dst_project))
        .collect_vec();
    if !requests.is_empty() {
        warn!("Existing pending request for destination project, doing nothing");
        return;
    }

    let req = Request::new_from_actions(
        client.clone(),
        "🤖: Importing from OBS".to_string(),
        actions,
    );
    if !args.dry_run {
        req.submit().await.unwrap();
    }
}
