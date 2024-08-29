use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ProwConfig {
    pub uri: String,
    pub consumer_type: String,
    pub consumer_number: String,
}
