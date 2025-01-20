use crate::server::state::RpcState;
use bridge_rpc::error::Web3Error;

#[derive(Debug)]
pub struct TestNamespace {
    pub state: RpcState,
}

impl Clone for TestNamespace {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl TestNamespace {
    pub fn new(state: RpcState) -> Self {
        Self { state }
    }
}

impl TestNamespace {
    pub async fn test(&self) -> Result<(), Web3Error> {
        self.state
            .test
            .test()
            .await
            .map_err(|_| Web3Error::InternalError)
    }
}
