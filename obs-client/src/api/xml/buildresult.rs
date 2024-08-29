use super::obs::{BuildArch, PackageCode, RepositoryCode};

#[derive(Debug, Clone, yaserde::YaDeserialize)]
#[yaserde(root = "resultlist")]
pub struct ResultList {
    pub result: Vec<RepositoryResult>,
}

#[derive(Debug, Clone, yaserde::YaDeserialize)]
pub struct RepositoryResult {
    #[yaserde(attribute)]
    pub repository: String,
    #[yaserde(attribute)]
    pub arch: BuildArch,
    #[yaserde(attribute)]
    pub code: RepositoryCode,
    pub summary: Option<Summary>,
    pub status: Vec<PackageStatus>,
    pub binarylist: Vec<PackageBinaryList>,
}

#[derive(Debug, Clone, yaserde::YaDeserialize)]
pub struct PackageStatus {
    #[yaserde(attribute)]
    pub package: String,
    #[yaserde(attribute)]
    pub code: PackageCode,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, yaserde::YaDeserialize)]
pub struct PackageBinaryList {
    #[yaserde(attribute)]
    pub package: String,
    pub binary: Vec<Binary>,
}

#[derive(Debug, Clone, yaserde::YaDeserialize)]
pub struct Binary {
    #[yaserde(attribute)]
    pub filename: String,
    #[yaserde(attribute)]
    pub size: u64,
    #[yaserde(attribute)]
    pub mtime: i64,
}

#[derive(Debug, Clone, yaserde::YaDeserialize)]
pub struct Summary {
    pub statuscount: Vec<StatusCount>,
}

#[derive(Debug, Clone, yaserde::YaDeserialize)]
pub struct StatusCount {
    #[yaserde(attribute)]
    pub code: PackageCode,
    #[yaserde(attribute)]
    pub count: u32,
}
