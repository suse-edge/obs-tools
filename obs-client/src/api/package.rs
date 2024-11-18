use std::fmt::Display;

use crate::error::APIError;

use super::{
    project::{Project, Repository},
    xml::buildinfo::BuildInfo,
    BuildArch,
};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Package {
    name: String,
    pub project: Project,
}

impl Package {
    pub fn from_name(name: String, project: Project) -> Self {
        Self { name, project }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub async fn release(&self, repository: &str, target: &Repository) -> Result<(), APIError> {
        let req = self
            .project
            .client
            .post(&["source", &self.project.name(), &self.name])
            .query(&[
                ("cmd", "release"),
                ("repository", repository),
                ("target_repository", target.name()),
                ("target_project", &target.project().name()),
            ])
            .build()
            .unwrap();
        self.project.client.execute(req).await?;
        Ok(())
    }

    pub async fn build_deps(
        &self,
        repository: Repository,
        architecture: BuildArch,
    ) -> Result<Vec<BinPackage>, APIError> {
        let req = self
            .project
            .client
            .get(&[
                "build",
                &self.project.name(),
                repository.name(),
                &format!("{}", architecture),
                &self.name,
                "_buildinfo",
            ])
            .build()
            .unwrap();
        let resp = self.project.client.execute(req).await?;
        let buildinfo: BuildInfo =
            yaserde::de::from_str(&resp.text().await?).map_err(APIError::XMLParseError)?;
        Ok(buildinfo
            .bdep
            .into_iter()
            .map(|b| BinPackage {
                name: b.name,
                version: b.version,
                release: b.release,
                arch: b.arch,
                repository: Repository::from_name_project(
                    &b.repository,
                    &Project::from_name(self.project.client.clone(), &b.project),
                ),
            })
            .collect())
    }
}

impl Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.project.name(), self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinPackage {
    pub name: String,
    pub version: String,
    pub release: String,
    pub arch: BuildArch,
    pub repository: Repository,
}
