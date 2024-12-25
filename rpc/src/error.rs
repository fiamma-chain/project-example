use thiserror::Error;

#[derive(Debug, Error)]
pub enum Web3Error {
    #[error("Internal error")]
    InternalError,
}
