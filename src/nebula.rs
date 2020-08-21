use serde_derive::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct NebulaConfig {
    pub repository: String,
}
