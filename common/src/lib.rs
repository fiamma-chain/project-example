use futures::{future, Future};
use tokio::{
    sync::oneshot,
    task::{JoinError, JoinHandle},
};

pub fn setup_sigint_handler() -> oneshot::Receiver<()> {
    let (sigint_sender, sigint_receiver) = oneshot::channel();
    let mut sigint_sender = Some(sigint_sender);
    ctrlc::set_handler(move || {
        if let Some(sigint_sender) = sigint_sender.take() {
            sigint_sender.send(()).ok();
            // ^ The send fails if `sigint_receiver` is dropped. We're OK with this,
            // since at this point the node should be stopping anyway, or is not interested
            // in listening to interrupt signals.
        }
    })
    .expect("Error setting Ctrl+C handler");

    sigint_receiver
}

pub async fn wait_for_tasks<Fut>(
    task_futures: Vec<JoinHandle<anyhow::Result<()>>>,
    graceful_shutdown: Option<Fut>,
    tasks_allowed_to_finish: bool,
) where
    Fut: Future<Output = ()>,
{
    match future::select_all(task_futures).await.0 {
        Ok(_) => {
            if tasks_allowed_to_finish {
                logs::error!("One of the actors finished its run. Finishing execution.");
            } else {
                logs::info!(
                    "One of the actors finished its run, while it wasn't expected to do it"
                );
                if let Some(graceful_shutdown) = graceful_shutdown {
                    graceful_shutdown.await;
                }
            }
        }
        Err(error) => {
            let panic_message = try_extract_panic_message(error);

            logs::error!(
                "One of the tokio actors unexpectedly finished with error: {panic_message}"
            );

            if let Some(graceful_shutdown) = graceful_shutdown {
                graceful_shutdown.await;
            }
        }
    }
}

pub fn try_extract_panic_message(err: JoinError) -> String {
    if err.is_panic() {
        let panic = err.into_panic();
        if let Some(panic_string) = panic.downcast_ref::<&'static str>() {
            panic_string.to_string()
        } else if let Some(panic_string) = panic.downcast_ref::<String>() {
            panic_string.to_string()
        } else {
            "Unknown panic".to_string()
        }
    } else {
        "Cancelled task".to_string()
    }
}
