use time::OffsetDateTime;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[non_exhaustive]
pub struct ContainerInfo {
    #[serde(with = "time::serde::timestamp")]
    pub buildtime: OffsetDateTime,
    pub disturl: Url,
    pub file: String,
    pub imageid: String,
    pub release: String,
    pub tags: Vec<String>,
    pub tar_blobids: Vec<String>,
    pub tar_manifest: String,
    pub tar_md5sum: String,
    #[serde(with = "time::serde::timestamp")]
    pub tar_mtime: OffsetDateTime,
    pub tar_sha256sum: String,
    pub tar_size: u64,
}
