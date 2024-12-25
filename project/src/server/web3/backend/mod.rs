use bridge_rpc::error::Web3Error;
use jsonrpsee::types::{error::ErrorCode, ErrorObjectOwned};

pub mod metadata;
pub mod middleware;
pub mod namespaces;

pub fn into_rpc_error(err: Web3Error) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(
        match err {
            Web3Error::InternalError => ErrorCode::InternalError.code(),
        },
        err.to_string(),
        None::<String>,
    )
}
