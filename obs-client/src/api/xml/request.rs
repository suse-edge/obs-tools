use yaserde::{YaDeserialize, YaSerialize};

#[derive(Debug, YaSerialize, YaDeserialize)]
#[yaserde(rename = "request")]
pub struct Request {
    pub action: Vec<Action>,
    pub description: String,
    #[yaserde(attribute)]
    pub id: Option<u32>,
}

#[derive(Debug, YaSerialize, YaDeserialize)]
pub struct Action {
    #[yaserde(attribute, rename = "type")]
    pub _type: String,
    pub source: Option<Target>,
    pub target: Target,
}

#[derive(Debug, YaSerialize, YaDeserialize)]
pub struct Target {
    #[yaserde(attribute)]
    pub project: String,
    #[yaserde(attribute)]
    pub package: String,
    #[yaserde(attribute)]
    pub rev: Option<u32>,
}

#[derive(Debug, YaDeserialize)]
pub struct Collection {
    pub request: Vec<Request>,
}
