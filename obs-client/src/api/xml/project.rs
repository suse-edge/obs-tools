use yaserde::YaDeserialize;

use super::{
    obs::{Flag, Group, Person, SimpleFlag},
    repository::Repository,
};

#[derive(Debug, Clone, YaDeserialize)]
pub struct Project {
    #[yaserde(attribute)]
    pub name: Option<String>,
    #[yaserde(attribute)]
    pub kind: Option<ProjectKind>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub link: Vec<ProjectLink>,
    #[yaserde(rename = "mountproject")]
    pub mount_project: Option<String>,
    #[yaserde(rename = "remoteurl")]
    pub remote_url: Option<String>,
    pub scmsync: Option<String>,
    pub devel: Option<DevelProject>,
    pub group: Vec<Group>,
    pub person: Vec<Person>,
    pub lock: Option<SimpleFlag>,
    pub build: Option<Flag>,
    pub publish: Option<Flag>,
    #[yaserde(rename = "useforbuild")]
    pub use_for_build: Option<Flag>,
    pub debuginfo: Option<Flag>,
    #[yaserde(rename = "binarydownload")]
    pub binary_download: Option<Flag>,
    #[yaserde(rename = "sourceaccess")]
    pub source_access: Option<SimpleFlag>,
    pub access: Option<SimpleFlag>,
    pub maintenance: Option<Maintenance>,
    pub repository: Vec<Repository>,
}

#[derive(Debug, Clone, YaDeserialize, PartialEq, Eq)]
pub enum ProjectKind {
    #[yaserde(rename = "standard")]
    Standard,
    #[yaserde(rename = "maintenance")]
    Maintenance,
    #[yaserde(rename = "maintenance_incident")]
    MaintenanceIncident,
    #[yaserde(rename = "maintenance_release")]
    MaintenanceRelease,
}
impl Default for ProjectKind {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Clone, YaDeserialize)]
pub struct Maintenance {
    pub maintains: Vec<Maintains>,
}

#[derive(Debug, Clone, YaDeserialize)]
pub struct Maintains {
    #[yaserde(attribute)]
    pub project: String,
}

#[derive(Debug, Clone, YaDeserialize)]
pub struct DevelProject {
    #[yaserde(attribute)]
    pub project: String,
}

#[derive(Debug, Clone, YaDeserialize)]
pub struct ProjectLink {
    #[yaserde(attribute)]
    pub project: String,
    #[yaserde(attribute)]
    pub vrevmode: VrevMode,
}

#[derive(Debug, Clone, YaDeserialize)]
pub enum VrevMode {
    Standard,
    Extend,
    Unextend,
}

impl Default for VrevMode {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Clone, YaDeserialize)]
pub struct SourceInfoList {
    pub sourceinfo: Vec<SourceInfo>,
}

#[derive(Debug, Clone, YaDeserialize)]
#[allow(dead_code)]
pub struct SourceInfo {
    #[yaserde(attribute)]
    pub package: String,
    #[yaserde(attribute, default)]
    pub rev: u32,
    #[yaserde(attribute)]
    pub vrev: u32,
    #[yaserde(attribute)]
    pub srcmd5: String,
    #[yaserde(attribute)]
    pub verifymd5: Option<String>,
    #[yaserde(attribute)]
    pub metamd5: Option<String>,

    pub filename: Option<String>,
    pub originproject: Option<String>,
    pub originpackage: Option<String>,
    pub linked: Vec<PackageLink>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub release: Option<String>,
    pub revtime: Option<String>,
    pub error: Option<String>,
    pub subpacks: Vec<String>,
    pub deps: Vec<String>,
    pub prereqs: Vec<String>,
}

#[derive(Debug, Clone, YaDeserialize)]
#[allow(dead_code)]
pub struct PackageLink {
    #[yaserde(attribute)]
    pub project: String,
    #[yaserde(attribute)]
    pub package: String,
}
