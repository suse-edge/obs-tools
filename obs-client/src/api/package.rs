use crate::error::APIError;

use super::project::{Project, Repository};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Package {
    name: String,
    project: Project,
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
}
