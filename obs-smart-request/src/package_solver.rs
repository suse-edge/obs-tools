use core::time;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    sync::Arc,
};

use itertools::Itertools;
use obs_client::api::{
    package::Package,
    project::{BinaryList, PackageInfo, Project},
    request::Action,
};
use tokio::sync::{mpsc::unbounded_channel, Mutex};
use tracing::{debug, info, warn};

pub async fn get_actions(
    existing_packages: Vec<PackageInfo>,
    base_packages: Vec<PackageInfo>,
    destination_project: &Project,
    fetch_deps_projects: Vec<String>,
) -> Vec<Action> {
    let existing_packages: HashMap<String, String> = existing_packages
        .into_iter()
        .map(|p| (p.package.name().to_string(), p.verifymd5))
        .collect();
    #[allow(clippy::mutable_key_type)]
    let mut actions: HashSet<Action> = HashSet::new();
    let (to_visit_sender, mut to_visit) = unbounded_channel::<PackageInfo>();
    for package in base_packages {
        to_visit_sender.send(package).unwrap();
    }
    // At the end of the run this set will contain all packages needed in remote repository
    let mut visited: HashSet<String> = HashSet::new();

    let solve_deps = !fetch_deps_projects.is_empty();
    let solver = DepsSolver {
        allowed_projects: fetch_deps_projects.clone(),
        ..Default::default()
    };
    while let Ok(package) = to_visit.try_recv() {
        if !visited.insert(package.package.name().to_string()) {
            // Already visited, skip it
            continue;
        }

        if existing_packages
            .get(package.package.name())
            .is_some_and(|dst_hash| *dst_hash == package.verifymd5)
        {
            debug!(src_package=%package.package, dst_project=destination_project.name(), "Package already here with same sources do not add to request");
        } else {
            info!(src_package=%package.package, dst_project=destination_project.name(), "Add this package to the request");
            actions.insert(Action::Submit {
                source: package.package.clone(),
                source_rev: Some(package.rev),
                target: Package::from_name(
                    package.package.name().to_string(),
                    destination_project.clone(),
                ),
            });
        }

        if solve_deps {
            let local_sender = to_visit_sender.clone();
            let local_solver = solver.clone();
            let local_package = package.package.clone();
            let deps = local_solver.solve_package(&local_package).await.into_iter();
            for dep in deps {
                local_sender.send(dep).unwrap();
            }
        }
    }
    debug!("Visited {} packages", visited.len());
    actions.into_iter().collect()
}

#[derive(Debug, Default, Clone)]
struct DepsSolver {
    binary_lists_cache: Arc<Mutex<HashMap<Project, BinaryList>>>,
    packageinfo_cache: Arc<Mutex<HashMap<Project, HashMap<Package, PackageInfo>>>>,
    allowed_projects: Vec<String>,
}

impl DepsSolver {
    async fn solve_package(&self, package: &Package) -> HashSet<PackageInfo> {
        tokio::time::sleep(time::Duration::from_millis(500)).await;
        let mut guard = self.binary_lists_cache.lock().await;
        if let Entry::Vacant(e) = guard.entry(package.project.clone()) {
            let bin = e.key().binarylist().await.unwrap();
            e.insert(bin);
        }
        let binlist = guard.get(&package.project).unwrap();
        #[allow(clippy::mutable_key_type)]
        let mut result = HashSet::default();
        let arch_reps = binlist
            .binaries
            .get(package)
            .unwrap()
            .iter()
            .flat_map(|(r, a)| a.keys().map(move |a| (r.clone(), a.clone())))
            .collect_vec();
        drop(guard);
        let futs = arch_reps.into_iter().map(|(r, a)| package.build_deps(r, a));
        let deps = futures::future::try_join_all(futs)
            .await
            .unwrap()
            .into_iter()
            .flatten()
            .filter(|b| {
                self.allowed_projects
                    .contains(&b.repository.project().name())
            });
        for dep in deps {
            let mut guard = self.binary_lists_cache.lock().await;
            let binlist = match guard.entry(dep.repository.project().clone()) {
                Entry::Vacant(e) => {
                    let bin = e.key().binarylist().await.unwrap();
                    e.insert(bin)
                }
                Entry::Occupied(e) => e.into_mut(),
            };
            let source = match binlist
                .binaries
                .iter()
                .find_map(|(p, r)| {
                    if let Some(a) = r.get(&dep.repository) {
                        if a.iter().any(|(_, bins)| {
                            bins.iter().any(|bin| {
                                bin.name
                                    == format!(
                                        "{}-{}-{}.{}.rpm",
                                        dep.name, dep.version, dep.release, dep.arch
                                    )
                            })
                        }) {
                            return Some(p.clone());
                        }
                    }
                    None
                })
                .clone() {
                    Some(p) => p,
                    None => {
                        warn!(?dep, "Unable to find source for dep");
                        continue;
                    }
                };
            drop(guard);

            result.insert(self.get_packageinfo(&source).await);
        }
        result
    }

    async fn get_packageinfo(&self, package: &Package) -> PackageInfo {
        let mut guard = self.packageinfo_cache.lock().await;
        #[allow(clippy::mutable_key_type)]
        let pkgs = match guard.entry(package.project.clone()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let pkgs = package
                    .project
                    .packagelist(false)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|p| (p.package.clone(), p));
                e.insert(pkgs.collect())
            }
        };
        pkgs.get(package).unwrap().clone()
    }
}
