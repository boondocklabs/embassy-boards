#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Deserialize(#[from] toml::de::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
