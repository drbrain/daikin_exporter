mod configuration;
mod daikin_adaptor;
mod daikin_exporter;
mod daikin_watcher;

use configuration::Configuration;
use daikin_exporter::DaikinExporter;
use daikin_watcher::DaikinWatcher;

use env_logger::Builder;
use env_logger::Env;

#[tokio::main]
async fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let configuration = Configuration::load_from_next_arg();

    let mut watcher = DaikinWatcher::new(&configuration);
    watcher.start();

    DaikinExporter::new(configuration.bind_address())
        .run(watcher)
        .await;
}
