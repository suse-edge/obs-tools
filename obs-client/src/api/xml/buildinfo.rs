use yaserde::YaDeserialize;

use crate::api::BuildArch;

#[non_exhaustive]
#[derive(Debug, YaDeserialize)]
pub struct BuildInfo {
    pub bdep: Vec<BDep>,
}

#[derive(Debug, YaDeserialize)]
pub struct BDep {
    #[yaserde(attribute)]
    pub name: String,
    #[yaserde(attribute)]
    pub version: String,
    #[yaserde(attribute)]
    pub release: String,
    #[yaserde(attribute)]
    pub arch: BuildArch,
    #[yaserde(attribute)]
    pub project: String,
    #[yaserde(attribute)]
    pub repository: String,
}
