use std::panic::AssertUnwindSafe;
use std::time::Duration;

use futures::FutureExt;
use tracing::{info, error};

pub async fn run_supervised<F, Fut>(name: &str, mut make_future: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    loop {
        let result = AssertUnwindSafe(make_future())
            .catch_unwind()
            .await;

        match result {
            Ok(()) => {
                info!("{name} task exited normally, restarting");
            }
            Err(panic) => {
                let msg = panic
                    .downcast_ref::<&str>()
                    .unwrap_or(&"unknown panic")
                    .to_string();
                error!("{name} task panicked: {msg}, restarting");
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
