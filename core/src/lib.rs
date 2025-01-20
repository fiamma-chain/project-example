use std::time::Instant;

use anyhow::Context;
use bitcoin::Network;
use config::api::{ApiConfig, HealthCheckConfig};
use dal::connection::{ConnectionPool, DbVariant};
use health_check::{healthcheck::HealthCheckHandle, CheckHealth};
use server::{ApiBuilder, Namespace};
use test::Test;
use tokio::{sync::watch, task::JoinHandle};

pub mod server;
pub mod test;

pub async fn genesis_init() -> anyhow::Result<()> {
    Ok(())
}

pub async fn initialize_tasks() -> anyhow::Result<(
    Vec<JoinHandle<anyhow::Result<()>>>,
    watch::Sender<bool>,
    HealthCheckHandle,
)> {
    let (stop_sender, stop_receiver) = watch::channel(false);
    let mut task_futures: Vec<JoinHandle<anyhow::Result<()>>> = vec![];
    let mut healthchecks: Vec<Box<dyn CheckHealth>> = Vec::new();
    let connection_pool = ConnectionPool::builder(DbVariant::Master).build().await;
    let api_config = ApiConfig::load_config().expect("failed to load api config");
    let test = Test::new();

    // Http server
    {
        let started_at = Instant::now();
        logs::info!("initializing HTTP API");

        let http_server_handles = ApiBuilder::jsonrpsee_backend(connection_pool.clone())
            .http(api_config.web3_json_rpc.http_port)
            .with_batch_request_size_limit(api_config.web3_json_rpc.max_batch_request_size())
            .with_response_body_size_limit(api_config.web3_json_rpc.max_response_body_size())
            .enable_api_namespaces(vec![Namespace::Bridge])
            .build()
            .context("failed to build HTTP JSON-RPC server")?
            .run(test.clone(), Network::Regtest, stop_receiver.clone())
            .await
            .context("Failed initializing HTTP JSON-RPC server")?;

        task_futures.extend(http_server_handles.tasks);
        healthchecks.push(Box::new(http_server_handles.health_check));
        logs::info!("initialized HTTP API in {:?}", started_at.elapsed());
    }
    // Pubsub server
    {
        let started_at = Instant::now();
        logs::info!("initializing PubsubApi API");

        let server_handles = ApiBuilder::pubsub_backend(connection_pool.clone())
            .ws(api_config.web3_json_rpc.ws_port)
            // .with_filters_limit(api_config.web3_json_rpc.filters_limit())
            // .with_subscriptions_limit(api_config.web3_json_rpc.subscriptions_limit())
            .with_batch_request_size_limit(api_config.web3_json_rpc.max_batch_request_size())
            .with_response_body_size_limit(api_config.web3_json_rpc.max_response_body_size())
            .with_polling_interval(api_config.web3_json_rpc.pubsub_interval())
            .with_threads(api_config.web3_json_rpc.ws_server_threads())
            .enable_api_namespaces(vec![Namespace::Pubsub])
            .build()
            .context("failed to build Websocket server")?
            .run(test.clone(), Network::Regtest, stop_receiver.clone())
            .await
            .context("run_pubsub_api")?;

        task_futures.extend(server_handles.tasks);
        healthchecks.push(Box::new(server_handles.health_check));
        logs::info!("initialized PubsubApi API in {:?}", started_at.elapsed());
    }

    let healtcheck_api_config =
        HealthCheckConfig::load_config().expect("failed to load health_check config");
    let health_check_handle =
        HealthCheckHandle::spawn_server(healtcheck_api_config.bind_addr(), healthchecks);

    Ok((task_futures, stop_sender, health_check_handle))
}
