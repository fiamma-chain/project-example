use std::time::Duration;

use common::{setup_sigint_handler, wait_for_tasks};
use dotenv::dotenv;
use logs::telemetry::{get_subscriber, init_subscriber, set_panic_hook};
use project_name::{genesis_init, initialize_tasks};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let (subscriber, _guard) = get_subscriber("bitvm-bridge".into(), "info".into());
    init_subscriber(subscriber);
    set_panic_hook();
    logs::info!("init_subscriber finished");

    genesis_init().await?;

    let sigint_receiver = setup_sigint_handler();

    let (core_task_handles, stop_sender, health_check_handle) = initialize_tasks()
        .await
        .expect("Failed to start bridge tasks");

    let graceful_shutdown = None::<futures::future::Ready<()>>;
    tokio::select! {
        _ = wait_for_tasks(core_task_handles, graceful_shutdown, false) => {},
        _ = sigint_receiver => {
            logs::info!("Stop signal received, shutting down");
        },
    }
    stop_sender.send(true).ok();
    tokio::time::sleep(Duration::from_secs(5)).await;
    health_check_handle.stop().await;
    logs::info!("Stopped");

    Ok(())
}
