use std::{net::SocketAddr, time::Duration};

use serde::Deserialize;

use crate::{load_config, BITVM_BRIDGE_PREFIX, BYTES_IN_MB};

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct ApiConfig {
    pub web3_json_rpc: Web3JsonRpcConfig,
    pub healthcheck: HealthCheckConfig,
    pub bitcoin_rpc: BitcoinRpcConfig,
}

impl ApiConfig {
    pub fn load_config() -> Result<ApiConfig, config::ConfigError> {
        Ok(ApiConfig {
            web3_json_rpc: Web3JsonRpcConfig::load_config()?,
            healthcheck: HealthCheckConfig::load_config()?,
            bitcoin_rpc: BitcoinRpcConfig::load_config()?,
        })
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct Web3JsonRpcConfig {
    pub http_port: u16,
    pub http_url: String,
    pub ws_port: u16,
    pub max_batch_request_size: Option<usize>,
    pub max_response_body_size_mb: Option<usize>,
    pub pubsub_polling_interval: Option<u64>,
    pub threads_per_server: u32,
}

impl Web3JsonRpcConfig {
    pub fn load_config() -> Result<Web3JsonRpcConfig, config::ConfigError> {
        load_config(
            "configuration/web3_json_rpc",
            format!("{BITVM_BRIDGE_PREFIX}_WEB3_JSON_RPC").as_str(),
        )
    }

    pub fn max_batch_request_size(&self) -> usize {
        self.max_batch_request_size.unwrap_or(500)
    }

    pub fn max_response_body_size(&self) -> usize {
        self.max_response_body_size_mb.unwrap_or(10) * BYTES_IN_MB
    }

    pub fn pubsub_interval(&self) -> Duration {
        Duration::from_secs(self.pubsub_polling_interval.unwrap_or(10))
    }

    pub fn ws_server_threads(&self) -> usize {
        self.threads_per_server as usize
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct HealthCheckConfig {
    /// Port to which the REST server is listening.
    pub port: u16,
}

impl HealthCheckConfig {
    pub fn load_config() -> Result<HealthCheckConfig, config::ConfigError> {
        load_config(
            "configuration/health_check",
            format!("{BITVM_BRIDGE_PREFIX}_HEALTHCHECK").as_str(),
        )
    }

    pub fn bind_addr(&self) -> SocketAddr {
        SocketAddr::new("0.0.0.0".parse().unwrap(), self.port)
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct BitcoinRpcConfig {
    pub http_url: String,
    pub rpc_user: String,
    pub rpc_password: String,
    pub confirms_threshold: u32,
}

impl BitcoinRpcConfig {
    pub fn load_config() -> Result<BitcoinRpcConfig, config::ConfigError> {
        load_config(
            "configuration/bitcoin_rpc",
            format!("{BITVM_BRIDGE_PREFIX}_BITCOIN").as_str(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiConfig, BitcoinRpcConfig, HealthCheckConfig, Web3JsonRpcConfig};
    use crate::utils::tests::EnvMutex;

    static MUTEX: EnvMutex = EnvMutex::new();

    fn default_config() -> ApiConfig {
        ApiConfig {
            web3_json_rpc: Web3JsonRpcConfig {
                http_port: 1001,
                ws_port: 1002,
                http_url: "http://127.0.0.1:1001".to_string(),
                max_batch_request_size: Some(200),
                max_response_body_size_mb: Some(10),
                pubsub_polling_interval: Some(10),
                threads_per_server: 128,
            },
            healthcheck: HealthCheckConfig { port: 33001 },
            bitcoin_rpc: BitcoinRpcConfig {
                http_url: "http://127.0.0.1:18443".to_string(),
                rpc_user: "test".to_string(),
                rpc_password: "1234".to_string(),
                confirms_threshold: 1,
            },
        }
    }

    #[test]
    fn test_load_api_config() {
        let mut lock = MUTEX.lock();
        let config = r#"
            BITVM_BRIDGE_WEB3_JSON_RPC_HTTP_PORT=1001
            BITVM_BRIDGE_WEB3_JSON_RPC_WS_PORT=1002
            BITVM_BRIDGE_WEB3_JSON_RPC_HTTP_URL=http://127.0.0.1:1001
            BITVM_BRIDGE_HEALTHCHECK_PORT=33001
            BITVM_BRIDGE_BITCOIN_RPC=http://127.0.0.1:18443
        "#;
        lock.set_env(config);

        let api_config = ApiConfig::load_config().expect("failed to load api config");
        assert_eq!(api_config, default_config());
    }
}
