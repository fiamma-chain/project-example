use bridge_rpc::error::Web3Error;

#[derive(Debug, Clone)]
pub struct Test {}

impl Test {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn test(&self) -> Result<(), Web3Error> {
        Ok(())
    }
}
