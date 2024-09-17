use std::sync::atomic::{AtomicU8, Ordering};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    sync::Arc,
};

use clap::Parser;
use console::{style, Emoji};
use dialoguer::{theme::ColorfulTheme, Confirm, MultiSelect};
use itertools::Itertools;
use obs_client::{
    api::{
        package::Package,
        project::{Binary, BinaryList, Project, ProjectKind, ReleaseTrigger, Repository},
        BuildArch,
    },
    files::{ContainerInfo, HelmInfo, Oscrc},
};
use url::Url;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short, long)]
    api_url: Option<Url>,
    #[arg(short, long)]
    username: Option<String>,
    #[arg(short, long)]
    password: Option<String>,
    project: String,

    #[arg(short, long, action=clap::ArgAction::Count)]
    verbose: u8,
    #[arg(short, long, action=clap::ArgAction::Count, conflicts_with="verbose")]
    quiet: u8,
}

static VERBOSITY: AtomicU8 = AtomicU8::new(2);
macro_rules! warn {
    ($fmt:literal $(, $arg:expr)*) => {
        if VERBOSITY.load(Ordering::Acquire) >= 2 {
            println!(concat!("{} ", $fmt), Emoji("⚠️", "!"), $(style($arg).bold()),*);
        }
    };
}
macro_rules! info {
    ($fmt:literal $(, $arg:expr)*) => {
        if VERBOSITY.load(Ordering::Acquire) >= 3 {
            println!(concat!("{} ", $fmt), Emoji("ℹ️", "i"), $(style($arg).bold()),*);
        }
    };
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::fmt()
        .pretty()
        .with_file(false)
        .with_line_number(false)
        .without_time()
        .with_target(false)
        .init();

    let args = Cli::parse();
    VERBOSITY.fetch_add(args.verbose, Ordering::AcqRel);
    VERBOSITY.fetch_sub(args.quiet, Ordering::AcqRel);
    let cfg = Oscrc::new(None).unwrap();
    let jar = obs_client::get_osc_cookiejar(&cfg).unwrap();
    let api_url = args.api_url.unwrap_or(cfg.apiurl.clone());
    let username = args
        .username
        .unwrap_or(cfg.hosts_options[&api_url].username.clone());

    let theme = ColorfulTheme::default();

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

    let project = obs_client::api::project::Project::from_name(client.clone(), &args.project);
    let meta = project.meta().await.unwrap();
    let release_targets: Vec<ReleaseTarget> = meta
        .repository
        .iter()
        .flat_map(|r| {
            r.releasetarget
                .iter()
                .filter(|t| t.trigger == ReleaseTrigger::Manual)
                .map(|t| ReleaseTarget {
                    src_repository: Repository::from_name_project(&r.name, &project),
                    dest_repository: Repository::from_name_project(
                        &t.repository,
                        &Project::from_name(client.clone(), &t.project),
                    ),
                })
        })
        .collect();

    for target in release_targets.iter() {
        info!("{}", target);
    }

    #[allow(clippy::mutable_key_type)]
    let mut binlist_release_project: HashMap<Project, BinaryList> = Default::default();
    let mut full_release_projects: Vec<Project> = Default::default();

    for project in release_targets
        .iter()
        .map(|t| t.dest_repository.project())
        .unique()
        .cloned()
        .collect_vec()
    {
        let meta = project.meta().await.unwrap();
        if meta
            .kind
            .is_some_and(|k| k == ProjectKind::MaintenanceRelease)
        {
            let binlist = project.binarylist().await.unwrap();
            binlist_release_project.insert(project, binlist);
        } else {
            full_release_projects.push(project);
        }
    }

    // Simple case, just release the project
    if binlist_release_project.is_empty() {
        if Confirm::with_theme(&theme)
            .with_prompt(format!("Fully release {} project ?", project.name()))
            .interact()
            .unwrap()
        {
            info!("Releasing {}", project.name());
            project.release().await.unwrap();
        } else {
            info!("Doing nothing !");
        }
        return;
    }

    let src_binlist = project.binarylist().await.unwrap();
    let mut packages_to_release: Vec<(Package, Repository, Repository)> = Default::default();

    for (package, bins) in src_binlist.binaries {
        let mut release_for_repo: Vec<(Repository, &Repository)> = Default::default();
        for (repo, bins) in bins {
            if bins.iter().all(|(_, b)| b.is_empty()) {
                continue;
            }
            for target in release_targets.iter().filter(|t| t.src_repository == repo) {
                if full_release_projects.contains(target.dest_repository.project()) {
                    release_for_repo.push((repo.clone(), &target.dest_repository));
                    continue;
                }
                let src_bins_info = get_bins_info(&bins).await;
                let src_tags: HashSet<String> =
                    src_bins_info.iter().flat_map(|b| b.tags.clone()).collect();
                let mut to_check = Vec::default();

                // Fetch binaries info and check for identical package already released
                for (pak, rep) in binlist_release_project[target.dest_repository.project()]
                    .binaries
                    .iter()
                    .filter(|(p, _)| {
                        let (base_name, _) = p.name().rsplit_once('.').unwrap_or(("", ""));
                        base_name == package.name()
                    })
                {
                    let dst_bins = &rep[&target.dest_repository];
                    let dst_bins_info = get_bins_info(dst_bins).await;
                    if src_bins_info == dst_bins_info {
                        info!("Package {} already released, skipping", package.name());
                        to_check = Vec::new();
                        break;
                    }
                    to_check.push((pak, dst_bins_info));
                }

                // Add package to release list and check for tags conflicts
                for (pak, dst_bins_info) in to_check {
                    release_for_repo.push((repo.clone(), &target.dest_repository));
                    let dst_tags: HashSet<String> =
                        dst_bins_info.iter().flat_map(|b| b.tags.clone()).collect();
                    for tag in dst_tags.intersection(&src_tags) {
                        warn!(
                            "{} will override tag {} set by {} in project {}",
                            package.name(),
                            tag,
                            pak.name(),
                            target.dest_repository.project().name()
                        );
                    }
                }
            }
        }
        packages_to_release.extend(
            release_for_repo
                .into_iter()
                .map(|(s, d)| (package.clone(), s, d.clone())),
        )
    }
    let mut select = MultiSelect::with_theme(&theme)
        .with_prompt("Select packages to release (press Esc to cancel):")
        .report(false);
    packages_to_release = packages_to_release.into_iter().unique().collect();
    let (max_package, max_src) =
        packages_to_release
            .iter()
            .fold((0, 0), |(acc_package, acc_src), (pack, src, _dst)| {
                (
                    acc_package.max(pack.name().len()),
                    acc_src.max(src.name().len()),
                )
            });
    for (package, src_rep, dst_rep) in packages_to_release.iter() {
        select = select.item_checked(
            format!(
                "{:<max_package$}\t{:>max_src$} -> {}/{}",
                package.name(),
                src_rep.name(),
                dst_rep.project().name(),
                dst_rep.name()
            ),
            true,
        );
    }
    let chosen = select.interact_opt().unwrap().unwrap_or_default();
    for (index, (package, src_rep, dst_rep)) in packages_to_release.into_iter().enumerate() {
        if chosen.contains(&index) {
            println!(
                "{} Releasing {} from {} to {}/{}",
                Emoji("📦", "->"),
                style(package.name()).bold(),
                style(src_rep.name()).bold(),
                style(dst_rep.project().name()).bold(),
                style(dst_rep.name()).bold(),
            );
            package.release(src_rep.name(), &dst_rep).await.unwrap();
        } else {
            info!(
                "Not releasing {} from {} to {}/{}",
                package.name(),
                src_rep.name(),
                dst_rep.project().name(),
                dst_rep.name()
            );
        }
    }
}

#[derive(Debug)]
struct ReleaseTarget {
    src_repository: Repository,
    dest_repository: Repository,
}

impl Display for ReleaseTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{} -> {}/{}",
            self.src_repository.project().name(),
            self.src_repository.name(),
            self.dest_repository.project().name(),
            self.dest_repository.name()
        )
    }
}

async fn get_bins_info(bins: &HashMap<BuildArch, Vec<Binary>>) -> HashSet<BinInfo> {
    let mut tags: HashSet<BinInfo> = Default::default();
    for bins in bins.values() {
        for bin in bins {
            match bin.name.rsplit_once('.') {
                None => continue,
                Some((_, "helminfo")) => {
                    let bin = bin.get().await.unwrap();
                    let helminfo: HelmInfo = serde_json::de::from_slice(&bin).unwrap();
                    tags.insert(BinInfo {
                        id: helminfo.chart_sha256,
                        tags: helminfo.tags,
                    });
                }
                Some((id, "rpm")) => {
                    tags.insert(BinInfo {
                        id: id.to_string(),
                        tags: Default::default(),
                    });
                }
                Some((_, "containerinfo")) => {
                    let bin = bin.get().await.unwrap();
                    let containerinfo: ContainerInfo = serde_json::de::from_slice(&bin).unwrap();
                    tags.insert(BinInfo {
                        id: containerinfo.imageid,
                        tags: containerinfo.tags,
                    });
                }
                Some(_) => continue,
            }
        }
    }
    tags
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct BinInfo {
    id: String,
    tags: Vec<String>,
}
