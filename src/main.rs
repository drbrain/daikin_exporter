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
use tokio::sync::oneshot;

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

    let exit_code = wait_for_error(error_rx).await;

    let result = DaikinExporter::new(configuration.bind_address())
        .run(watcher)
        .await;

    if let Err(e) = result {
        match error_tx.send(e).await {
            Ok(_) => (),
            Err(_) => unreachable!("Error channel closed unexpectedly, bug?"),
        }
    }

    std::process::exit(exit_code.await.unwrap_or(2));
}

// Wait for an error and signal when it is received.  The error code will always be 1 because this
// program never exits normally, but we need to make sure an error reported from a failure in
// DaikinExporter / prometheus_exporter is logged properly so this returns a oneshot::Receiver.
//
// This would be more straightforward if prometheus_exporter were implemented atop tokio (or maybe
// some other way) but it's not (or I don't know how).
// If there were a tokio-capable prometheus_exporter then both the error handler and the prometheus
// exporter web server could be joined/selected together.
async fn wait_for_error(mut error_rx: mpsc::Receiver<anyhow::Error>) -> oneshot::Receiver<i32> {
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let error = match error_rx.recv().await {
            Some(e) => e,
            None => anyhow!("Error reporting channel closed unexpectedly, bug?"),
        };

        error!("{:#}", error);

        tx.send(1).unwrap();
    });

    rx
}
