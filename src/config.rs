use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub interface: String,
}

impl Settings {
    pub fn new() -> anyhow::Result<Self> {
        Ok(config::Config::builder()
            .add_source(config::File::with_name("settings"))
            .build()?
            .try_deserialize()?)
    }
}
