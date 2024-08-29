use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use obs_client::client::OBSClient;
use serde::Serialize;
use tokio::net::TcpListener;
use url::Url;

use crate::dispatcher::Trigger;

#[derive(Clone)]
struct WebAppState {
    obs_client: Arc<OBSClient>,
    trigger: Arc<dyn Trigger>,
    projects: Vec<String>,
}

pub async fn run_web_app(
    obs_client: Arc<OBSClient>,
    trigger: Arc<dyn Trigger>,
    projects: Vec<String>,
) -> anyhow::Result<()> {
    let state = WebAppState {
        obs_client,
        trigger,
        projects,
    };
    let app = Router::new()
        .route("/projects", get(get_projects))
        .route("/projects/:name", get(get_project).post(post_project))
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct GetProjectsResponse {
    projects: Vec<String>,
}

async fn get_projects(State(state): State<WebAppState>) -> Json<GetProjectsResponse> {
    Json(GetProjectsResponse {
        projects: state.projects,
    })
}

#[derive(Debug, Serialize)]
struct GetProjectResponse {
    name: String,
    ready: bool,
    monitor_url: Url,
}

async fn get_project(
    State(state): State<WebAppState>,
    Path(project): Path<String>,
) -> Result<Json<GetProjectResponse>, StatusCode> {
    if !state.projects.contains(&project) {
        return Err(StatusCode::NOT_FOUND);
    }
    let project = obs_client::api::project::Project::from_name(state.obs_client.clone(), &project);
    let summary = match project.summary().await {
        Ok(p) => p,
        Err(_) => {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let monitor_url = state
        .obs_client
        .get_obs_route(&["project", "monitor", &project.name()])
        .await;
    Ok(Json(GetProjectResponse {
        name: project.name(),
        ready: summary.is_all_packages_ok() && summary.is_all_published(),
        monitor_url,
    }))
}

async fn post_project(State(state): State<WebAppState>, Path(project): Path<String>) -> StatusCode {
    if !state.projects.contains(&project) {
        return StatusCode::NOT_FOUND;
    }

    let project = obs_client::api::project::Project::from_name(state.obs_client, &project);
    let summary = match project.summary().await {
        Ok(p) => p,
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };
    if !(summary.is_all_packages_ok() && summary.is_all_published()) {
        return StatusCode::CONFLICT;
    }
    match state.trigger.call(&project.name()).await {
        Ok(()) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
