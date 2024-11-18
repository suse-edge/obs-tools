use std::sync::Arc;

use crate::{client::OBSClient, error::APIError};

use super::package::Package;
use super::project::Project;
use super::xml::request::{Action as XMLAction, Collection, Request as XMLRequest, Target};

use itertools::Itertools;

#[derive(Debug)]
pub struct Request {
    client: Arc<OBSClient>,
    pub actions: Vec<Action>,
    pub description: String,
    id: Option<u32>,
}

impl Request {
    pub fn new_from_actions(
        client: Arc<OBSClient>,
        description: String,
        actions: Vec<Action>,
    ) -> Self {
        Self {
            client,
            actions,
            description,
            id: None,
        }
    }

    pub async fn submit(self) -> Result<(), APIError> {
        let body = XMLRequest {
            description: self.description,
            action: self.actions.iter().map_into().collect(),
            id: None,
        };
        let req = self
            .client
            .post(&["request"])
            .query(&[("cmd", "create")])
            .body(yaserde::ser::to_string(&body).map_err(APIError::XMLParseError)?)
            .build()?;
        self.client.execute(req).await?;
        Ok(())
    }

    pub async fn mine(client: Arc<OBSClient>) -> Result<Vec<Self>, APIError> {
        let req = client
            .get(&["request"])
            .query(&[
                ("view", "collection"),
                ("roles", "creator"),
                ("states", "new,review"),
                ("user", client.user()),
            ])
            .build()?;
        let resp = client.execute(req).await?;
        let collection: Collection =
            yaserde::de::from_str(&resp.text().await?).map_err(APIError::XMLParseError)?;
        Ok(collection
            .request
            .into_iter()
            .map(|r| Request {
                client: client.clone(),
                actions: r
                    .action
                    .into_iter()
                    .map(|a| {
                        let target_package = Package::from_name(
                            a.target.package,
                            Project::from_name(client.clone(), &a.target.project),
                        );
                        match a._type.as_str() {
                            "delete" => Action::Delete(target_package),
                            "submit" => {
                                let source = a.source.unwrap();
                                let source_package = Package::from_name(
                                    source.package,
                                    Project::from_name(client.clone(), &source.project),
                                );
                                Action::Submit {
                                    source: source_package,
                                    source_rev: source.rev,
                                    target: target_package,
                                }
                            }
                            _ => panic!("unknow action type"),
                        }
                    })
                    .collect(),
                description: r.description,
                id: r.id,
            })
            .collect())
    }

    pub fn is_for_project(&self, project: &Project) -> bool {
        self.actions.iter().any(|a| a.is_for_project(project))
    }

    pub async fn delete(&self) -> Result<(), APIError> {
        let req = self
            .client
            .delete(&[
                "request",
                &self.id.ok_or(APIError::InvalidObject)?.to_string(),
            ])
            .build()?;
        self.client.execute(req).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Action {
    Submit {
        source: Package,
        source_rev: Option<u32>,
        target: Package,
    },
    Delete(Package),
}

impl From<&Action> for XMLAction {
    fn from(value: &Action) -> Self {
        match value {
            Action::Delete(p) => Self {
                _type: "delete".to_string(),
                source: None,
                target: Target {
                    package: p.name().to_string(),
                    project: p.project.name(),
                    rev: None,
                },
            },
            Action::Submit {
                source,
                source_rev,
                target,
            } => Self {
                _type: "submit".to_string(),
                source: Some(Target {
                    package: source.name().to_string(),
                    project: source.project.name(),
                    rev: *source_rev,
                }),
                target: Target {
                    rev: None,
                    package: target.name().to_string(),
                    project: target.project.name(),
                },
            },
        }
    }
}

impl Action {
    pub fn is_for_project(&self, project: &Project) -> bool {
        match self {
            Action::Delete(p) => p.project == *project,
            Action::Submit {
                source: _,
                source_rev: _,
                target,
            } => target.project == *project,
        }
    }
}
