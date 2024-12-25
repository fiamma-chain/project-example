use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[cfg_attr(
    all(feature = "client", feature = "server"),
    rpc(server, client, namespace = "test")
)]
#[cfg_attr(
    all(feature = "client", not(feature = "server")),
    rpc(client, namespace = "test")
)]
#[cfg_attr(
    all(not(feature = "client"), feature = "server"),
    rpc(server, namespace = "test")
)]
pub trait TestNamespace {
    #[method(name = "test")]
    async fn test(&self) -> RpcResult<()>;
}
