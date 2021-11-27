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

use log::error;

use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
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
        .start()
        .await;

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
