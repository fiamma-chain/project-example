use std::time::Duration;

use bitcoin::Network;
use futures::FutureExt;
use jsonrpsee::{
    core::server::SubscriptionMessage,
    server::IdProvider,
    types::{error::ErrorCode, ErrorObject, SubscriptionId},
    PendingSubscriptionSink, SendTimeoutError, SubscriptionSink,
};
use types::pubsub::PubSubResult;

use dal::connection::ConnectionPool;
use tokio::{
    self,
    sync::{broadcast, mpsc, watch},
    task::JoinHandle,
    time::interval,
};
use web3::types::H128;

pub mod rpc;

const BROADCAST_CHANNEL_CAPACITY: usize = 8192;
const SUBSCRIPTION_SINK_SEND_TIMEOUT: Duration = Duration::from_secs(180);
pub const EVENT_TOPIC_NUMBER_LIMIT: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct EthSubscriptionIdProvider;

impl IdProvider for EthSubscriptionIdProvider {
    fn next_id(&self) -> SubscriptionId<'static> {
        let id = H128::random();
        format!("0x{}", hex::encode(id.0)).into()
    }
}

/// Events emitted by the subscription logic. Only used in WebSocket server tests so far.
#[derive(Debug)]
pub(super) enum PubSubEvent {
    Subscribed(SubscriptionType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum SubscriptionType {
    Test,
}

/// Manager of notifications for a certain type of subscriptions.
#[derive(Debug)]
struct PubSubNotifier {
    sender: broadcast::Sender<Vec<PubSubResult>>,
    _connection_pool: ConnectionPool,
    polling_interval: Duration,
    _events_sender: Option<mpsc::UnboundedSender<PubSubEvent>>,
    _network: Network,
}

impl PubSubNotifier {
    fn _emit_event(&self, event: PubSubEvent) {
        if let Some(sender) = &self._events_sender {
            sender.send(event).ok();
        }
    }
}

impl PubSubNotifier {
    fn send_pub_sub_results(&self, results: Vec<PubSubResult>, _sub_type: SubscriptionType) {
        // Errors only on 0 receivers, but we want to go on if we have 0 subscribers so ignore the error.
        self.sender.send(results).ok();
    }

    // broadcast presign task for committee.
    async fn notify_new_task(self, stop_receiver: watch::Receiver<bool>) -> anyhow::Result<()> {
        let mut timer = interval(self.polling_interval);
        loop {
            if *stop_receiver.borrow() {
                tracing::info!("Stop signal received, pubsub_logs_notifier is shutting down");
                break;
            }
            timer.tick().await;

            let new_tasks = self.new_task().await?;

            if new_tasks.len() > 0 {
                logs::info!("{} new tasks send successfully", new_tasks.len());
                let tasks = new_tasks
                    .iter()
                    .map(|task| PubSubResult::Syncing(task.clone()))
                    .collect();
                self.send_pub_sub_results(tasks, SubscriptionType::Test);

                logs::info!("after send pegin committee tasks");
            } else {
                logs::info!("no new pegin committee tasks");
            }
        }
        Ok(())
    }

    async fn new_task(&self) -> anyhow::Result<Vec<bool>> {
        Ok(vec![true, false, true])
    }
}

pub(super) struct TestSubscribe {
    _connection_pool: ConnectionPool,
    // task senders
    tests: broadcast::Sender<Vec<PubSubResult>>,
    events_sender: Option<mpsc::UnboundedSender<PubSubEvent>>,
    network: Network,
}

impl TestSubscribe {
    pub fn new(connection_pool: ConnectionPool, network: Network) -> Self {
        let (tests, _) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);

        Self {
            _connection_pool: connection_pool,
            tests,
            events_sender: None,
            network,
        }
    }

    async fn reject(sink: PendingSubscriptionSink) {
        sink.reject(ErrorObject::borrowed(
            ErrorCode::InvalidParams.code(),
            &"Rejecting subscription - invalid parameters provided.",
            None,
        ))
        .await;
    }

    // receiver: broadcast receiver
    async fn _run_subscriber(
        sink: SubscriptionSink,
        subscription_type: SubscriptionType,
        mut receiver: broadcast::Receiver<Vec<PubSubResult>>,
    ) {
        let closed = sink.closed().fuse();
        tokio::pin!(closed);

        loop {
            tokio::select! {
                new_items_result = receiver.recv() => {
                    let new_items = match new_items_result {
                        Ok(items) => items,
                        Err(broadcast::error::RecvError::Closed) => {
                            // The broadcast channel has closed because the notifier task is shut down.
                            // This is fine; we should just stop this task.
                            logs::error!("subscription_type {:?} closed", subscription_type);
                            break;
                        }
                        Err(broadcast::error::RecvError::Lagged(message_count)) => {
                            logs::error!("skipped_broadcast_message {:?} count {:?}", subscription_type, message_count);
                            match receiver.try_recv() {
                                Ok(latest_items) => latest_items,
                                Err(_) => continue, // No messages available, wait for next one
                            }
                        }
                    };

                    logs::info!("new_items {:?} count {:?}", subscription_type, new_items.len());

                    let handle_result = Self::_handle_new_items(
                        &sink,
                        subscription_type,
                        new_items,
                    )
                    .await;
                    if handle_result.is_err() {
                        logs::error!("subscriber_send_timeouts {:?} error {:?}", subscription_type, handle_result);
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        continue;
                    }
                }
                _ = &mut closed => {
                    logs::info!("run_subscriber {:?} closed", subscription_type);
                }
            }
        }
        logs::info!("run_subscriber {:?} finished", subscription_type);
    }

    async fn _handle_new_items(
        sink: &SubscriptionSink,
        subscription_type: SubscriptionType,
        new_items: Vec<PubSubResult>,
    ) -> Result<(), SendTimeoutError> {
        for item in new_items {
            sink.send_timeout(
                SubscriptionMessage::from_json(&item)
                    .expect("PubSubResult always serializable to json;qed"),
                SUBSCRIPTION_SINK_SEND_TIMEOUT,
            )
            .await?;

            logs::info!("notify {:?}", subscription_type);
        }

        logs::info!("notify {:?} new items finished", subscription_type);
        Ok(())
    }

    #[logs::instrument(name = "sub_bridge", skip(self, pending_sink))]
    pub async fn sub(&self, pending_sink: PendingSubscriptionSink, sub_type: String) {
        logs::info!("sub {:?}", sub_type);
        let sub_type = match sub_type.as_str() {
            "syncing" => {
                let Ok(sink) = pending_sink.accept().await else {
                    return;
                };

                tokio::spawn(async move {
                    sink.send_timeout(
                        SubscriptionMessage::from_json(&PubSubResult::Syncing(false)).unwrap(),
                        SUBSCRIPTION_SINK_SEND_TIMEOUT,
                    )
                    .await
                });
                None
            }
            _ => {
                Self::reject(pending_sink).await;
                None
            }
        };

        if let Some(sub_type) = sub_type {
            if let Some(sender) = &self.events_sender {
                sender.send(PubSubEvent::Subscribed(sub_type)).ok();
            }
        }
    }

    /// Spawns notifier tasks. This should be called once per instance.
    pub fn spawn_notifiers(
        &self,
        connection_pool: ConnectionPool,
        polling_interval: Duration,
        stop_receiver: watch::Receiver<bool>,
    ) -> Vec<JoinHandle<anyhow::Result<()>>> {
        let mut notifier_tasks = Vec::with_capacity(2);

        let tests = PubSubNotifier {
            sender: self.tests.clone(),
            _connection_pool: connection_pool.clone(),
            polling_interval,
            _events_sender: self.events_sender.clone(),
            _network: self.network,
        };
        let tests_task = tokio::spawn(tests.notify_new_task(stop_receiver.clone()));

        notifier_tasks.push(tests_task);

        notifier_tasks
    }
}
