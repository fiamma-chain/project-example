use std::panic;

use tracing::{subscriber::set_global_default, Subscriber};
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::time::ChronoUtc;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};

pub fn get_subscriber(
    name: String,
    env_filter: String,
) -> (impl Subscriber + Send + Sync, WorkerGuard) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let environment = std::env::var("BITVM_BRIDGE_ENVIRONMENT").unwrap_or("local".to_string());
    let with_ansi = environment != "local";
    let mut base_path = std::env::current_dir().expect("Failed to determine the current directory");
    base_path.push(".logs");
    base_path.push(&name);
    let file_appender = rolling::RollingFileAppender::builder()
        .filename_prefix("{{project-name}}")
        .filename_suffix("log")
        .rotation(rolling::Rotation::HOURLY)
        .build(base_path)
        .expect("Failed to create file appender");

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_timer(ChronoUtc::new("%Y-%m-%dT%H:%M:%S%.3fZ".to_string()))
        .with_ansi(with_ansi)
        .with_level(true);

    let res = Registry::default().with(env_filter).with(file_layer);
    (res, guard)
}

/// Register a subscriber as global default to process span data.
///
/// It should only be called once!
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    // Redirect all `log`'s events to our subscriber
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

pub fn set_panic_hook() {
    let placeholder = "Unknown panic info".to_string();
    panic::set_hook(Box::new(move |panic_info| {
        let payload = panic_info
            .payload()
            .downcast_ref::<String>()
            .unwrap_or(&placeholder);
        let location = panic_info
            .location()
            .unwrap_or_else(|| panic::Location::caller());
        let panic_message = format!(
            "Panic occurred in file '{}' at line {}: {}",
            location.file(),
            location.line(),
            payload
        );

        super::error!("{}", panic_message);
    }));
}
