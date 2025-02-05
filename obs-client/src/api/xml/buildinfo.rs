use yaserde::YaDeserialize;

use crate::api::BuildArch;

#[non_exhaustive]
#[derive(Debug, YaDeserialize)]
pub struct BuildInfo {
    pub bdep: Vec<BDep>,
}

#[derive(Debug, YaDeserialize)]
pub struct BDep {
    #[yaserde(attribute = true)]
    pub name: String,
    #[yaserde(attribute = true)]
    pub version: String,
    #[yaserde(attribute = true)]
    pub release: String,
    #[yaserde(attribute = true)]
    pub arch: BuildArch,
    #[yaserde(attribute = true)]
    pub project: String,
    #[yaserde(attribute = true)]
    pub repository: String,
}
