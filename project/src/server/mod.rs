use std::{collections::HashSet, net::SocketAddr, num::NonZeroU32, sync::Arc, time::Duration};

use anyhow::Context;
use bitcoin::Network;
use bridge_rpc::namespaces::{pubsub::TestPubSubServer, test::TestNamespaceServer};
use dal::connection::ConnectionPool;
use futures::future;
use health_check::{HealthStatus, HealthUpdater, ReactiveHealthCheck};
use jsonrpsee::{
    server::{BatchRequestConfig, PingConfig, RpcServiceBuilder, ServerBuilder},
    RpcModule,
};
use pubsub::TestSubscribe;
use serde::Deserialize;
use state::RpcState;
use tokio::{
    sync::{oneshot, watch},
    task::JoinHandle,
};
use tower_http::cors::CorsLayer;
use web3::{
    backend::{metadata::MethodTracer, middleware::LimitMiddleware},
    namespaces::test::TestNamespace,
};

use crate::test::Test;

pub mod pubsub;
pub mod state;
pub mod web3;

#[derive(Debug, Clone, Copy)]
enum ApiTransport {
    WebSocket(SocketAddr),
    Http(SocketAddr),
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Namespace {
    Bridge,
    Pubsub,
}

impl Namespace {
    pub const DEFAULT: &'static [Namespace] = &[Namespace::Bridge];
}

/// Handles to the initialized API server.
#[derive(Debug)]
pub struct ApiServerHandles {
    pub tasks: Vec<JoinHandle<anyhow::Result<()>>>,
    pub health_check: ReactiveHealthCheck,
    #[allow(unused)] // only used in tests
    pub(crate) local_addr: future::TryMaybeDone<oneshot::Receiver<SocketAddr>>,
}

/// Optional part of the API server parameters.
#[derive(Debug, Default)]
struct OptionalApiParams {
    filters_limit: Option<usize>,
    subscriptions_limit: Option<usize>,
    batch_request_size_limit: Option<usize>,
    response_body_size_limit: Option<usize>,
    websocket_requests_per_minute_limit: Option<NonZeroU32>,
    threads: Option<usize>,
}

#[derive(Debug)]
pub struct ApiServer {
    pool: ConnectionPool,
    health_updater: Arc<HealthUpdater>,
    transport: ApiTransport,
    polling_interval: Duration,
    namespaces: Vec<Namespace>,
    method_tracer: Arc<MethodTracer>,
    optional: OptionalApiParams,
}

#[derive(Debug)]
pub struct ApiBuilder {
    pool: ConnectionPool,
    transport: Option<ApiTransport>,
    polling_interval: Duration,
    namespaces: Option<Vec<Namespace>>,
    method_tracer: Arc<MethodTracer>,
    optional: OptionalApiParams,
}

impl ApiBuilder {
    const DEFAULT_POLLING_INTERVAL: Duration = Duration::from_secs(10);

    pub fn jsonrpsee_backend(pool: ConnectionPool) -> Self {
        Self {
            pool,
            polling_interval: Self::DEFAULT_POLLING_INTERVAL,
            transport: None,
            namespaces: None,
            method_tracer: Arc::new(MethodTracer::default()),
            optional: OptionalApiParams::default(),
        }
    }

    pub fn pubsub_backend(pool: ConnectionPool) -> Self {
        Self {
            pool,
            polling_interval: Self::DEFAULT_POLLING_INTERVAL,
            transport: None,
            namespaces: None,
            method_tracer: Arc::new(MethodTracer::default()),
            optional: OptionalApiParams::default(),
        }
    }

    pub fn http(mut self, port: u16) -> Self {
        self.transport = Some(ApiTransport::Http(([0, 0, 0, 0], port).into()));
        self
    }

    pub fn ws(mut self, port: u16) -> Self {
        self.transport = Some(ApiTransport::WebSocket(([0, 0, 0, 0], port).into()));
        self
    }

    pub fn with_filters_limit(mut self, filters_limit: usize) -> Self {
        self.optional.filters_limit = Some(filters_limit);
        self
    }

    pub fn with_subscriptions_limit(mut self, subscriptions_limit: usize) -> Self {
        self.optional.subscriptions_limit = Some(subscriptions_limit);
        self
    }

    pub fn with_batch_request_size_limit(mut self, limit: usize) -> Self {
        self.optional.batch_request_size_limit = Some(limit);
        self
    }

    pub fn with_response_body_size_limit(mut self, limit: usize) -> Self {
        self.optional.response_body_size_limit = Some(limit);
        self
    }

    pub fn with_websocket_requests_per_minute_limit(
        mut self,
        websocket_requests_per_minute_limit: NonZeroU32,
    ) -> Self {
        self.optional.websocket_requests_per_minute_limit =
            Some(websocket_requests_per_minute_limit);
        self
    }

    pub fn with_polling_interval(mut self, polling_interval: Duration) -> Self {
        self.polling_interval = polling_interval;
        self
    }

    pub fn enable_api_namespaces(mut self, namespaces: Vec<Namespace>) -> Self {
        self.namespaces = Some(namespaces);
        self
    }

    pub fn with_threads(mut self, threads: usize) -> Self {
        self.optional.threads = Some(threads);
        self
    }

    #[cfg(test)]
    fn with_method_tracer(mut self, method_tracer: Arc<MethodTracer>) -> Self {
        self.method_tracer = method_tracer;
        self
    }
}

impl ApiBuilder {
    pub fn build(self) -> anyhow::Result<ApiServer> {
        let transport = self.transport.context("API transport not set")?;
        let health_check_name = match &transport {
            ApiTransport::Http(_) => "http_api",
            ApiTransport::WebSocket(_) => "ws_api",
        };
        let (_, health_updater) = ReactiveHealthCheck::new(health_check_name);

        Ok(ApiServer {
            pool: self.pool,
            health_updater: Arc::new(health_updater),
            transport,
            polling_interval: self.polling_interval,
            namespaces: self.namespaces.unwrap_or_else(|| {
                logs::warn!(
                    "debug_ and snapshots_ API namespace will be disabled by default in ApiBuilder"
                );
                Namespace::DEFAULT.to_vec()
            }),
            method_tracer: self.method_tracer,
            optional: self.optional,
        })
    }
}

impl ApiServer {
    pub fn health_check(&self) -> ReactiveHealthCheck {
        self.health_updater.subscribe()
    }

    async fn build_rpc_state(self, test: Test) -> anyhow::Result<RpcState> {
        Ok(RpcState {
            _current_method: self.method_tracer,
            _connection_pool: self.pool,
            test,
        })
    }

    async fn build_rpc_module(
        self,
        test: Test,
        pub_sub: Option<TestSubscribe>,
    ) -> anyhow::Result<RpcModule<()>> {
        let namespaces = self.namespaces.clone();

        let mut rpc = RpcModule::new(());
        if let Some(pub_sub) = pub_sub {
            rpc.merge(pub_sub.into_rpc())
                .expect("Can't merge eth pubsub namespace");
        }
        if namespaces.contains(&Namespace::Bridge) {
            let rpc_state = self.build_rpc_state(test).await?;
            rpc.merge(TestNamespace::new(rpc_state).into_rpc())
                .expect("Can't merge Committee namespace");
        }

        Ok(rpc)
    }

    pub async fn run(
        self,
        test: Test,
        network: Network,
        stop_receiver: watch::Receiver<bool>,
    ) -> anyhow::Result<ApiServerHandles> {
        if self.optional.filters_limit.is_none() {
            logs::warn!("Filters limit is not set - unlimited filters are allowed");
        }

        match (&self.transport, self.optional.subscriptions_limit) {
            (ApiTransport::WebSocket(_), None) => {
                logs::warn!(
                    "`subscriptions_limit` is not set - unlimited subscriptions are allowed"
                );
            }
            (ApiTransport::Http(_), Some(_)) => {
                logs::warn!(
                    "`subscriptions_limit` is ignored for HTTP transport, use WebSocket instead"
                );
            }
            _ => {}
        }

        self.build_jsonrpsee(test, network, stop_receiver).await
    }

    async fn build_jsonrpsee(
        self,
        test: Test,
        network: Network,
        stop_receiver: watch::Receiver<bool>,
    ) -> anyhow::Result<ApiServerHandles> {
        let mut tasks = vec![];
        let pubsub = if matches!(self.transport, ApiTransport::WebSocket(_))
            && self.namespaces.contains(&Namespace::Pubsub)
        {
            let pubsub = TestSubscribe::new(self.pool.clone(), network);
            tasks.extend(pubsub.spawn_notifiers(
                self.pool.clone(),
                self.polling_interval,
                stop_receiver.clone(),
            ));
            logs::info!("Pubsub server started");
            Some(pubsub)
        } else {
            None
        };
        // Start the server in a separate tokio runtime from a dedicated thread.
        let health_check = self.health_updater.subscribe();
        let (local_addr_sender, local_addr) = oneshot::channel();
        let server_task =
            tokio::spawn(self.run_jsonrpsee_server(test, pubsub, stop_receiver, local_addr_sender));

        tasks.push(server_task);
        Ok(ApiServerHandles {
            health_check,
            tasks,
            local_addr: future::try_maybe_done(local_addr),
        })
    }

    async fn run_jsonrpsee_server(
        self,
        test: Test,
        pubsub: Option<TestSubscribe>,
        mut stop_receiver: watch::Receiver<bool>,
        local_addr_sender: oneshot::Sender<SocketAddr>,
    ) -> anyhow::Result<()> {
        let transport = self.transport;
        let (transport_str, is_http, addr) = match transport {
            ApiTransport::Http(addr) => ("HTTP", true, addr),
            ApiTransport::WebSocket(addr) => ("WS", false, addr),
        };

        let batch_request_config = self
            .optional
            .batch_request_size_limit
            .map_or(BatchRequestConfig::Unlimited, |limit| {
                BatchRequestConfig::Limit(limit as u32)
            });
        let response_body_size_limit = self
            .optional
            .response_body_size_limit
            .map_or(u32::MAX, |limit| limit as u32);
        let websocket_requests_per_minute_limit = self.optional.websocket_requests_per_minute_limit;
        let subscriptions_limit = self.optional.subscriptions_limit;
        let health_updater = self.health_updater.clone();

        let rpc = self.build_rpc_module(test, pubsub).await?;
        let registered_method_names = Arc::new(rpc.method_names().collect::<HashSet<_>>());
        logs::debug!(
            "Built RPC module for {transport_str} server with {} methods: {registered_method_names:?}",
            registered_method_names.len()
        );

        // Setup CORS.
        let cors = is_http.then(|| {
            CorsLayer::new()
                // Allow `POST` when accessing the resource
                .allow_methods([reqwest::Method::POST])
                // Allow requests from any origin
                .allow_origin(tower_http::cors::Any)
                .allow_headers([reqwest::header::CONTENT_TYPE])
        });

        // Assemble server middleware.
        let middleware = tower::ServiceBuilder::new().option_layer(cors);

        // Settings shared by HTTP and WS servers.
        let max_connections = !is_http
            .then_some(subscriptions_limit)
            .flatten()
            .unwrap_or(5_000);

        #[allow(clippy::let_and_return)] // simplifies conditional compilation
        let rpc_middleware = RpcServiceBuilder::new()
            .layer_fn(move |svc| svc)
            .option_layer((!is_http).then(|| {
                tower::layer::layer_fn(move |svc| {
                    LimitMiddleware::new(svc, websocket_requests_per_minute_limit)
                })
            }));

        let ping_config = PingConfig::default();
        ping_config.inactive_limit(Duration::from_secs(120));
        ping_config.ping_interval(Duration::from_secs(30));
        ping_config.max_failures(3);

        let server_builder = ServerBuilder::default()
            .max_connections(max_connections as u32)
            .set_http_middleware(middleware)
            .max_response_body_size(response_body_size_limit)
            .set_batch_request_config(batch_request_config)
            .set_rpc_middleware(rpc_middleware)
            .enable_ws_ping(ping_config);

        let (local_addr, server_handle) = if is_http {
            // HTTP-specific settings
            let server = server_builder
                .http_only()
                .build(addr)
                .await
                .context("Failed building HTTP JSON-RPC server")?;
            (server.local_addr(), server.start(rpc))
        } else {
            // WS specific settings
            let server = server_builder
                .build(addr)
                .await
                .context("Failed building WS JSON-RPC server")?;
            (server.local_addr(), server.start(rpc))
        };
        let local_addr = local_addr.with_context(|| {
            format!("Failed getting local address for {transport_str} JSON-RPC server")
        })?;
        logs::info!("Initialized {transport_str} API on {local_addr:?}");
        local_addr_sender.send(local_addr).ok();
        health_updater.update(HealthStatus::Ready.into());

        // We want to be able to immediately stop the server task if the server stops on its own for whatever reason.
        // Hence, we monitor `stop_receiver` on a separate Tokio task.
        let close_handle = server_handle.clone();
        // We use `Weak` reference to the health updater in order to not prevent its drop if the server stops on its own.
        // (QIT-26): While `Arc<HealthUpdater>` is stored in `self`, we rely on the fact that `self` is consumed and
        // dropped by `self.build_rpc_module` above, so we should still have just one strong reference.
        let closing_health_updater = Arc::downgrade(&health_updater);
        tokio::spawn(async move {
            if stop_receiver.changed().await.is_err() {
                logs::warn!(
                    "Stop signal sender for {transport_str} JSON-RPC server was dropped \
                     without sending a signal"
                );
            }
            if let Some(health_updater) = closing_health_updater.upgrade() {
                health_updater.update(HealthStatus::ShuttingDown.into());
            }
            logs::info!("Stop signal received, {transport_str} JSON-RPC server is shutting down");
            close_handle.stop().ok();
        });
        server_handle.stopped().await;
        drop(health_updater);

        logs::info!("{transport_str} JSON-RPC server stopped");
        Ok(())
    }
}
