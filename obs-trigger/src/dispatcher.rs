use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use obs_client::client::OBSClient;
use tokio::sync::Mutex;
use tracing::{debug, info, span};

use crate::gangway::prow_client::ProwClient;
use crate::gangway::{
    CreateJobExecutionRequest, JobExecutionStatus, JobExecutionType, ListJobExecutionsRequest,
};
use crate::{Action, Check, Project};

#[async_trait]
pub trait Trigger: Send + Sync {
    async fn call(&self, project: &str) -> anyhow::Result<()>;
}

pub struct Dispatcher<T> {
    projects: HashMap<String, Project>,
    obs_client: Arc<OBSClient>,
    prow_client: Option<Arc<Mutex<ProwClient<T>>>>,
}

impl<T> Dispatcher<T> {
    pub fn new(
        projects: HashMap<String, Project>,
        obs_client: Arc<OBSClient>,
        prow_client: Option<Arc<Mutex<ProwClient<T>>>>,
    ) -> Self {
        Dispatcher {
            projects,
            obs_client,
            prow_client,
        }
    }

    pub fn get_projects_list(&self) -> Vec<String> {
        self.projects.keys().cloned().collect()
    }
}

#[async_trait]
impl<T> Trigger for Dispatcher<T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody> + Send + Sync,
    T::Future: Send,
    T::Error: Into<tonic::codegen::StdError>,
    T::ResponseBody: tonic::codegen::Body<Data = tonic::codegen::Bytes> + Send + 'static,
    <T::ResponseBody as tonic::codegen::Body>::Error: Into<tonic::codegen::StdError> + Send,
{
    async fn call(&self, project: &str) -> anyhow::Result<()> {
        let _ = span!(tracing::Level::DEBUG, "Handling trigger", ?project).entered();
        let project_conf = self
            .projects
            .get(project)
            .ok_or(anyhow::anyhow!("No such project"))?;
        debug!("Checking pre-requisites");
        for check in project_conf.checks.iter() {
            match check {
                Check::ProwJob { name } => {
                    let prow_client_mutex = self
                        .prow_client
                        .clone()
                        .ok_or(anyhow::anyhow!("Got Prow action and no prow config"))?;
                    let mut prow_client = prow_client_mutex.lock().await;
                    let pending_jobs = prow_client
                        .list_job_executions(ListJobExecutionsRequest {
                            job_name: name.clone(),
                            status: JobExecutionStatus::Pending.into(),
                        })
                        .await?
                        .into_inner()
                        .job_execution;
                    let triggered_jobs = prow_client
                        .list_job_executions(ListJobExecutionsRequest {
                            job_name: name.clone(),
                            status: JobExecutionStatus::Triggered.into(),
                        })
                        .await?
                        .into_inner()
                        .job_execution;
                    if !(pending_jobs.is_empty() && triggered_jobs.is_empty()) {
                        info!(
                            ?pending_jobs,
                            ?triggered_jobs,
                            "Running or pending Prow jobs, skipping run"
                        );
                        return Ok(());
                    }
                }
            }
        }

        debug!("Trigger actions");
        for action in project_conf.actions.iter() {
            match action {
                Action::ObsRelease => {
                    let project = obs_client::api::project::Project::from_name(
                        self.obs_client.clone(),
                        project,
                    );
                    project.release().await?;
                }
                Action::ProwJob { name, job_type } => {
                    let prow_client_mutex = self
                        .prow_client
                        .clone()
                        .ok_or(anyhow::anyhow!("Got Prow action and no prow config"))?;
                    let mut prow_client = prow_client_mutex.lock().await;
                    prow_client
                        .create_job_execution(CreateJobExecutionRequest {
                            job_name: name.clone(),
                            job_execution_type: JobExecutionType::from_str_name(job_type)
                                .ok_or(anyhow::anyhow!("Non valid job type given"))?
                                .into(),
                            refs: None,
                            pod_spec_options: Default::default(),
                        })
                        .await?;
                }
            }
        }
        Ok(())
    }
}
