use super::obs::{BuildArch, PackageCode, RepositoryCode};

#[derive(Debug, Clone, yaserde::YaDeserialize)]
#[yaserde(rename = "resultlist")]
pub struct ResultList {
    pub result: Vec<RepositoryResult>,
}

#[derive(Debug, Clone, yaserde::YaDeserialize)]
pub struct RepositoryResult {
    #[yaserde(attribute = true)]
    pub repository: String,
    #[yaserde(attribute = true)]
    pub arch: BuildArch,
    #[yaserde(attribute = true)]
    pub code: RepositoryCode,
    pub summary: Option<Summary>,
    pub status: Vec<PackageStatus>,
    pub binarylist: Vec<PackageBinaryList>,
}

#[derive(Debug, Clone, yaserde::YaDeserialize)]
pub struct PackageStatus {
    #[yaserde(attribute = true)]
    pub package: String,
    #[yaserde(attribute = true)]
    pub code: PackageCode,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, yaserde::YaDeserialize)]
pub struct PackageBinaryList {
    #[yaserde(attribute = true)]
    pub package: String,
    pub binary: Vec<Binary>,
}

#[derive(Debug, Clone, yaserde::YaDeserialize)]
pub struct Binary {
    #[yaserde(attribute = true)]
    pub filename: String,
    #[yaserde(attribute = true)]
    pub size: u64,
    #[yaserde(attribute = true)]
    pub mtime: i64,
}

#[derive(Debug, Clone, yaserde::YaDeserialize)]
pub struct Summary {
    pub statuscount: Vec<StatusCount>,
}

#[derive(Debug, Clone, yaserde::YaDeserialize)]
pub struct StatusCount {
    #[yaserde(attribute = true)]
    pub code: PackageCode,
    #[yaserde(attribute = true)]
    pub count: u32,
}
