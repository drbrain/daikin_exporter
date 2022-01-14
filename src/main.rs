mod configuration;
mod daikin_adaptor;
mod daikin_discover;
mod daikin_exporter;
mod daikin_watcher;

use configuration::Configuration;
use daikin_discover::DaikinDiscover;
use daikin_exporter::DaikinExporter;
use daikin_watcher::DaikinWatcher;

use anyhow::anyhow;
use anyhow::Result;

use env_logger::Builder;
use env_logger::Env;

use lazy_static::lazy_static;

use log::error;

use prometheus::register_gauge;
use prometheus::Gauge;

use tokio::signal::ctrl_c;
use tokio::sync::mpsc;

use std::time::SystemTime;
use std::time::UNIX_EPOCH;

lazy_static! {
    static ref START_TIME: Gauge = register_gauge!(
        "process_start_time_seconds",
        "Start time of the process since unix epoch in seconds."
    )
    .unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    let start_time = SystemTime::now().duration_since(UNIX_EPOCH).ok();

    Builder::from_env(Env::default().default_filter_or("info")).init();

    let configuration = Configuration::load_from_next_arg();

    let (error_tx, error_rx) = mpsc::channel(1);

    let discover = DaikinDiscover::new(&configuration)
        .await?
        .start(error_tx.clone())
        .await;

    let mut watcher = DaikinWatcher::new(discover, &configuration);
    watcher.start().await;

    DaikinExporter::new(configuration.bind_address())?
        .start(error_tx.clone())
        .await;

    if let Some(duration) = start_time {
        START_TIME.set(duration.as_secs_f64());
    }

    tokio::spawn(async {
        ctrl_c().await.unwrap();

        std::process::exit(0);
    });

    let exit_code = wait_for_error(error_rx).await;

    std::process::exit(exit_code);
}

async fn wait_for_error(mut error_rx: mpsc::Receiver<anyhow::Error>) -> i32 {
    let error = match error_rx.recv().await {
        Some(e) => e,
        None => anyhow!("Error reporting channel closed unexpectedly, bug?"),
    };

    error!("{:#}", error);

    1
}
