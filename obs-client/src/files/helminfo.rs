use time::OffsetDateTime;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct HelmInfo {
    pub name: String,
    pub version: String,
    pub release: String,
    pub tags: Vec<String>,
    pub disturl: Url,
    #[serde(with = "time::serde::timestamp")]
    pub buildtime: OffsetDateTime,
    pub chart: String,
    pub config_json: String,
    pub chart_sha256: String,
    pub chart_size: u64,
}
