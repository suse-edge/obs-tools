use std::collections::HashMap;
use std::sync::Arc;

use itertools::Itertools;
use time::OffsetDateTime;

use crate::client::OBSClient;
use crate::error::APIError;

use super::package::Package;
use super::xml::buildepinfo::BuildDepInfo;
use super::xml::buildresult::{ResultList, Summary};
use super::xml::obs::{BuildArch, PackageCode, RepositoryCode};

use super::xml::project::SourceInfoList;
pub use super::xml::project::{Project as ProjectMeta, ProjectKind};
pub use super::xml::repository::ReleaseTrigger;

#[derive(Debug, thiserror::Error)]
#[error("Wrong ResultList kind provided")]
pub struct ResultError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageInfo {
    pub package: Package,
    pub rev: u32,
    pub vrev: u32,
    pub srcmd5: String,
    pub verifymd5: String,
    pub version: Option<String>,
    pub subpackages: Vec<String>,
}

#[derive(Debug)]
pub struct Binary {
    pub name: String,
    pub size: u64,
    pub mtime: OffsetDateTime,
    pub package: Package,
    pub repository: Repository,
    pub architecture: BuildArch,
}

impl Binary {
    pub async fn get(&self) -> Result<Vec<u8>, APIError> {
        let req = self
            .repository
            .project
            .client
            .get(&[
                "build",
                &self.repository.project.name,
                &self.repository.name,
                &self.architecture.to_string(),
                self.package.name(),
                &self.name,
            ])
            .build()?;
        let resp = self
            .repository
            .project
            .client
            .execute(req)
            .await?
            .error_for_status()?;
        Ok(resp.bytes().await?.to_vec())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Repository {
    name: String,
    project: Project,
}

impl Repository {
    pub fn from_name_project(name: &str, project: &Project) -> Self {
        Self {
            name: name.to_string(),
            project: project.clone(),
        }
    }
    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub async fn deps_tree(
        &self,
        arch: BuildArch,
    ) -> Result<HashMap<Arc<Package>, Vec<Arc<Package>>>, APIError> {
        let req = self
            .project
            .client
            .get(&["build", &self.project.name, &self.name, &arch.to_string()])
            .build()?;
        let resp = self.project.client.execute(req).await?;
        let deps: BuildDepInfo =
            yaserde::de::from_str(&resp.text().await?).map_err(APIError::XMLParseError)?;
        #[allow(clippy::mutable_key_type)]
        let mut packages: HashMap<Arc<Package>, Vec<Arc<Package>>> = deps
            .package
            .iter()
            .map(|p| {
                (
                    Arc::new(Package::from_name(p.name.clone(), self.project.clone())),
                    vec![],
                )
            })
            .collect();
        for dep in deps.package {
            let deps = dep
                .pkgdep
                .into_iter()
                .map(|d| packages.keys().find(|p| p.name() == d).unwrap().clone())
                .collect_vec();
            if let std::collections::hash_map::Entry::Occupied(mut entry) =
                packages.entry(Arc::new(Package::from_name(dep.name, self.project.clone())))
            {
                entry.insert(deps);
            } else {
                return Err(APIError::InvalidObject);
            }
        }
        Ok(packages)
    }
}

#[derive(Debug)]
pub struct BinaryList {
    pub binaries: HashMap<Package, HashMap<Repository, HashMap<BuildArch, Vec<Binary>>>>,
}

impl BinaryList {
    fn from_result_list(project: Project, result_list: ResultList) -> Result<Self, ResultError> {
        #[allow(clippy::mutable_key_type)]
        let mut binaries: HashMap<
            Package,
            HashMap<Repository, HashMap<BuildArch, Vec<Binary>>>,
        > = Default::default();
        for repo in result_list.result {
            let repository = Repository {
                name: repo.repository,
                project: project.clone(),
            };
            if repo.binarylist.is_empty() {
                return Err(ResultError {});
            }
            for package in repo.binarylist {
                if package.binary.is_empty() {
                    continue;
                }
                let pack = Package::from_name(package.package, project.clone());
                #[allow(clippy::mutable_key_type)]
                let out_pack = binaries.entry(pack.clone()).or_default();
                let out_repo = out_pack
                    .entry(repository.clone())
                    .or_default()
                    .entry(repo.arch.clone())
                    .or_default();
                for binary in package.binary {
                    out_repo.push(Binary {
                        name: binary.filename,
                        size: binary.size,
                        mtime: OffsetDateTime::from_unix_timestamp(binary.mtime).unwrap(),
                        package: pack.clone(),
                        repository: repository.clone(),
                        architecture: repo.arch.clone(),
                    });
                }
            }
        }
        Ok(Self { binaries })
    }
}

pub struct ProjectSummary {
    repositories: HashMap<(String, BuildArch), RepositorySummary>,
}

impl ProjectSummary {
    pub fn is_all_published(&self) -> bool {
        self.repositories.iter().all(|(_, s)| s.is_published())
    }

    pub fn is_all_packages_ok(&self) -> bool {
        self.repositories.iter().all(|(_, s)| s.is_packages_ok())
    }

    pub fn get(&self, repository: String, arch: BuildArch) -> Option<&RepositorySummary> {
        self.repositories.get(&(repository, arch))
    }

    pub fn iter(
        &self,
    ) -> std::collections::hash_map::Iter<'_, (String, BuildArch), RepositorySummary> {
        self.repositories.iter()
    }
}

impl IntoIterator for ProjectSummary {
    type Item = ((String, BuildArch), RepositorySummary);
    type IntoIter = std::collections::hash_map::IntoIter<(String, BuildArch), RepositorySummary>;

    fn into_iter(self) -> Self::IntoIter {
        self.repositories.into_iter()
    }
}

pub struct RepositorySummary {
    code: RepositoryCode,
    counts: HashMap<PackageCode, u32>,
}

impl RepositorySummary {
    pub fn is_published(&self) -> bool {
        self.code == RepositoryCode::Published
    }

    pub fn is_packages_ok(&self) -> bool {
        self.counts
            .iter()
            .all(|(code, count)| *count == 0 || code.is_ok())
    }

    fn from_summary(code: RepositoryCode, summary: Summary) -> Self {
        Self {
            code,
            counts: summary
                .statuscount
                .into_iter()
                .map(|s| (s.code, s.count))
                .collect(),
        }
    }
}

impl TryFrom<ResultList> for ProjectSummary {
    type Error = ResultError;
    fn try_from(value: ResultList) -> Result<Self, Self::Error> {
        let repositories = value
            .result
            .into_iter()
            .map(|r| {
                Ok::<_, Self::Error>((
                    (r.repository, r.arch),
                    RepositorySummary::from_summary(r.code, r.summary.ok_or(ResultError {})?),
                ))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { repositories })
    }
}

impl std::fmt::Display for ProjectSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ((repo, arch), summary) in self.repositories.iter() {
            writeln!(f, "{}/{:?} ({:?})", repo, arch, summary.code)?;
            for (code, count) in summary.counts.iter() {
                writeln!(f, "\t{:?}: {}", code, count)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Project {
    pub(crate) client: Arc<OBSClient>,
    name: String,
}

impl Project {
    pub fn from_name(client: Arc<OBSClient>, name: &str) -> Self {
        Project {
            name: name.to_string(),
            client,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub async fn release(&self) -> Result<(), APIError> {
        let req = self
            .client
            .post(&["source", &self.name])
            .query(&[("cmd", "release")])
            .build()?;
        self.client.execute(req).await?;
        Ok(())
    }

    pub async fn summary(&self) -> Result<ProjectSummary, APIError> {
        let req = self
            .client
            .get(&["build", &self.name, "_result"])
            .query(&[("view", "summary")])
            .build()?;
        let resp = self.client.execute(req).await?;
        let xlm_resp: ResultList =
            yaserde::de::from_str(&resp.text().await?).map_err(APIError::XMLParseError)?;
        Ok(xlm_resp.try_into().unwrap())
    }

    pub async fn binarylist(&self) -> Result<BinaryList, APIError> {
        let req = self
            .client
            .get(&["build", &self.name, "_result"])
            .query(&[("view", "binarylist")])
            .build()?;
        let resp = self.client.execute(req).await?;
        Ok(BinaryList::from_result_list(
            self.clone(),
            yaserde::de::from_str(&resp.text().await?).map_err(APIError::XMLParseError)?,
        )
        .unwrap())
    }

    pub async fn packagelist(&self, full: bool) -> Result<Vec<PackageInfo>, APIError> {
        let req = self.client.get(&["source", &self.name]);
        let req = match full {
            true => req.query(&[("view", "info"), ("parse", "1")]),
            false => req.query(&[("view", "info"), ("nofilename", "1")]),
        }
        .build()?;
        let resp = self.client.execute(req).await?;
        let list: SourceInfoList =
            yaserde::de::from_str(&resp.text().await?).map_err(APIError::XMLParseError)?;
        Ok(list
            .sourceinfo
            .iter()
            .map(|s| PackageInfo {
                package: Package::from_name(s.package.clone(), self.clone()),
                rev: s.rev,
                vrev: s.vrev,
                srcmd5: s.srcmd5.clone(),
                verifymd5: s.verifymd5.clone().unwrap_or_default(),
                version: s.version.clone(),
                subpackages: s.subpacks.clone(),
            })
            .collect())
    }

    pub async fn meta(&self) -> Result<ProjectMeta, APIError> {
        let req = self.client.get(&["source", &self.name, "_meta"]).build()?;
        let resp = self.client.execute(req).await?;
        yaserde::de::from_str(&resp.text().await?).map_err(APIError::XMLParseError)
    }
}
