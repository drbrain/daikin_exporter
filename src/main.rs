mod configuration;
mod daikin_adaptor;
mod daikin_discover;
mod daikin_exporter;
mod daikin_watcher;

use configuration::Configuration;
use daikin_discover::DaikinDiscover;
use daikin_exporter::DaikinExporter;
use daikin_watcher::DaikinWatcher;

use anyhow::Result;

use env_logger::Builder;
use env_logger::Env;

#[tokio::main]
async fn main() -> Result<()> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let configuration = Configuration::load_from_next_arg();

    let (discover, _daikin_error_rx) = DaikinDiscover::new(&configuration).await?.start().await;

    let mut watcher = DaikinWatcher::new(discover, &configuration);
    watcher.start().await;

    DaikinExporter::new(configuration.bind_address())
        .run(watcher)
        .await;

    Ok(())
}
