use bridge_rpc::{
    jsonrpsee::http_client::{HttpClient, HttpClientBuilder},
    namespaces::test::TestNamespaceClient,
};
use error::ClientError;

pub mod error;
pub mod provider;
pub mod utils;

pub struct Wallet<P> {
    pub provider: P,
}

impl Wallet<HttpClient> {
    pub fn with_http_client(rpc_address: &str) -> Result<Wallet<HttpClient>, ClientError> {
        let client = HttpClientBuilder::default().build(rpc_address)?;

        Ok(Wallet { provider: client })
    }
}

impl<P> Wallet<P>
where
    P: TestNamespaceClient + Sync,
{
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    pub async fn test(&self) -> anyhow::Result<()> {
        self.provider
            .test()
            .await
            .map_err(|e| anyhow::anyhow!("failed to test: {}", e.to_string()))
    }
}
