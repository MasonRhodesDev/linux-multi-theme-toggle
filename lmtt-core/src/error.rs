use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Config error: {0}")]
    Config(String),
    
    #[error("Matugen error: {0}")]
    Matugen(String),
    
    #[error("Module error: {0}")]
    Module(String),
    
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
