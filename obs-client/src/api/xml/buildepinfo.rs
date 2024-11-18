use yaserde::YaDeserialize;

#[derive(Debug, Clone, YaDeserialize)]
pub struct BuildDepInfo {
    pub package: Vec<DepPackage>,
}

#[derive(Debug, Clone, YaDeserialize)]
pub struct DepPackage {
    #[yaserde(attribute)]
    pub name: String,
    pub pkgdep: Vec<String>,
}
