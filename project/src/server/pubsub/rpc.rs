use bridge_rpc::namespaces::pubsub::TestPubSubServer;
use jsonrpsee::{
    core::{RpcResult, SubscriptionResult},
    PendingSubscriptionSink,
};

use super::TestSubscribe;

#[async_trait::async_trait]
impl TestPubSubServer for TestSubscribe {
    async fn subscribe(
        &self,
        pending: PendingSubscriptionSink,
        sub_type: String,
    ) -> SubscriptionResult {
        self.sub(pending, sub_type).await;
        Ok(())
    }

    async fn send_message(&self, _message: String) -> RpcResult<()> {
        Ok(())
    }
}
