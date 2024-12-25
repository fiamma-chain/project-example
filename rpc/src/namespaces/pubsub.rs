use jsonrpsee::{
    core::{RpcResult, SubscriptionResult},
    proc_macros::rpc,
};

#[rpc(server, namespace = "test")]
pub trait TestPubSub {
    #[subscription(name = "subscribe" => "subscription", unsubscribe = "unsubscribe", item = PubSubResult)]
    async fn subscribe(&self, sub_type: String) -> SubscriptionResult;

    #[method(name = "sendMessage")]
    async fn send_message(&self, message: String) -> RpcResult<()>;
}
