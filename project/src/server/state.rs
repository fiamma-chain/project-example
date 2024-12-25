use std::sync::Arc;

use dal::connection::ConnectionPool;

use crate::test::Test;

use super::web3::backend::metadata::MethodTracer;

#[derive(Debug, Clone)]
pub struct RpcState {
    pub(super) _current_method: Arc<MethodTracer>,
    pub(super) _connection_pool: ConnectionPool,
    pub test: Test,
}
