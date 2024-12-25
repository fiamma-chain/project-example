use bridge_rpc::jsonrpsee::core::ClientError as RpcError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Missing required field for a transaction: {0}")]
    MissingRequiredField(String),
    #[error("RPC error: {0:?}")]
    RpcError(#[from] RpcError),
    #[error("Invalid ABI File")]
    AbiParseError,
}
