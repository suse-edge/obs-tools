use yaserde::YaDeserialize;

use super::obs::BuildArch;

#[derive(Debug, Clone, YaDeserialize)]
pub struct Repository {
    #[yaserde(attribute)]
    pub name: String,
    #[yaserde(attribute)]
    pub rebuild: Option<RebuildMode>,
    #[yaserde(attribute)]
    pub block: Option<BlockMode>,
    #[yaserde(attribute)]
    pub linkedbuild: Option<LinkedBuildMode>,
    pub download: Vec<Download>,
    pub releasetarget: Vec<ReleaseTarget>,
    pub hostsystem: Vec<Path>,
    pub path: Vec<Path>,
    pub arch: Vec<BuildArch>,
}

#[derive(Debug, Clone, YaDeserialize)]
pub enum RebuildMode {
    #[yaserde(rename = "transitive")]
    Transitive,
    #[yaserde(rename = "direct")]
    Direct,
    #[yaserde(rename = "local")]
    Local,
}

impl Default for RebuildMode {
    fn default() -> Self {
        Self::Transitive
    }
}

#[derive(Debug, Clone, YaDeserialize)]
pub enum BlockMode {
    #[yaserde(rename = "all")]
    All,
    #[yaserde(rename = "local")]
    Local,
    #[yaserde(rename = "never")]
    Never,
}

impl Default for BlockMode {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Debug, Clone, YaDeserialize)]
pub enum LinkedBuildMode {
    #[yaserde(rename = "off")]
    Off,
    #[yaserde(rename = "localdep")]
    Localdep,
    #[yaserde(rename = "alldirect")]
    Alldirect,
    #[yaserde(rename = "all")]
    All,
}

impl Default for LinkedBuildMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone, YaDeserialize)]
pub struct Path {
    #[yaserde(attribute)]
    pub project: String,
    #[yaserde(attribute)]
    pub repository: String,
}

#[derive(Debug, Clone, YaDeserialize)]
pub struct ReleaseTarget {
    #[yaserde(attribute)]
    pub project: String,
    #[yaserde(attribute)]
    pub repository: String,
    #[yaserde(attribute)]
    pub trigger: ReleaseTrigger,
}

#[derive(Debug, Clone, YaDeserialize, PartialEq)]
pub enum ReleaseTrigger {
    None,
    #[yaserde(rename = "manual")]
    Manual,
    #[yaserde(rename = "maintenance")]
    Maintenance,
    #[yaserde(rename = "obsgendiff")]
    Obsgendiff,
}

impl Default for ReleaseTrigger {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, YaDeserialize)]
pub struct Download {
    #[yaserde(attribute)]
    pub arch: String,
    #[yaserde(attribute)]
    pub url: String,
    #[yaserde(attribute)]
    pub repotype: RepoType,
    pub archfilter: Option<String>,
    pub master: Option<Master>,
    pub pubkey: Option<String>,
}

#[derive(Debug, Clone, YaDeserialize)]
pub enum RepoType {
    #[yaserde(rename = "rpmmd")]
    Rpmmd,
    #[yaserde(rename = "susetags")]
    Susetags,
    #[yaserde(rename = "deb")]
    Deb,
    #[yaserde(rename = "arch")]
    Arch,
    #[yaserde(rename = "mdk")]
    Mdk,
    #[yaserde(rename = "registry")]
    Registry,
}

// There are no real default here, but yaserde wants one
impl Default for RepoType {
    fn default() -> Self {
        Self::Rpmmd
    }
}

#[derive(Debug, Clone, YaDeserialize)]
pub struct Master {
    pub url: String,
    pub sslfingerprint: Option<String>,
}
