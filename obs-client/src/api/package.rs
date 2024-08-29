use super::project::Project;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Package {
    name: String,
    project: Project,
}

impl Package {
    pub fn from_name(name: String, project: Project) -> Self {
        Self { name, project }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
