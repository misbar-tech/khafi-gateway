use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid proof: {0}")]
    InvalidProof(String),

    #[error("Nullifier replay detected")]
    NullifierReplay,

    #[error("Serialization error: {0}")]
    BincodeEncode(#[from] bincode::error::EncodeError),

    #[error("Deserialization error: {0}")]
    BincodeDecode(#[from] bincode::error::DecodeError),

    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("RISC Zero error: {0}")]
    RiscZero(String),

    #[error("Compilation error: {0}")]
    Compilation(String),

    #[error("DSL parsing error: {0}")]
    DslParsing(String),

    #[error("Zcash error: {0}")]
    Zcash(String),

    #[error("Invalid nullifier format")]
    InvalidNullifier,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
