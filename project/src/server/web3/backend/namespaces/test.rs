use bridge_rpc::namespaces::test::TestNamespaceServer;
use jsonrpsee::core::{async_trait, RpcResult};

use crate::server::web3::namespaces::test::TestNamespace;

#[async_trait]
impl TestNamespaceServer for TestNamespace {
    async fn test(&self) -> RpcResult<()> {
        Ok(())
    }
}
