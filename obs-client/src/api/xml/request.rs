use yaserde::{YaDeserialize, YaSerialize};

#[derive(Debug, YaSerialize, YaDeserialize)]
#[yaserde(rename = "request")]
pub struct Request {
    pub action: Vec<Action>,
    pub description: String,
    #[yaserde(attribute = true)]
    pub id: Option<u32>,
}

#[derive(Debug, YaSerialize, YaDeserialize)]
pub struct Action {
    #[yaserde(attribute = true, rename = "type")]
    pub _type: String,
    pub source: Option<Target>,
    pub target: Target,
}

#[derive(Debug, YaSerialize, YaDeserialize)]
pub struct Target {
    #[yaserde(attribute = true)]
    pub project: String,
    #[yaserde(attribute = true)]
    pub package: String,
    #[yaserde(attribute = true)]
    pub rev: Option<u32>,
}

#[derive(Debug, YaDeserialize)]
pub struct Collection {
    pub request: Vec<Request>,
}
