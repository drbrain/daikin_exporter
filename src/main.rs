mod configuration;
mod daikin_adaptor;
mod daikin_exporter;

use configuration::Configuration;
use daikin_adaptor::DaikinAdaptor;
use daikin_exporter::DaikinExporter;

use env_logger::Builder;
use env_logger::Env;

use log::info;

use reqwest::Client;

use std::time::Duration;

fn new_client(timeout: Duration) -> Client {
    Client::builder()
        .connect_timeout(timeout)
        .http1_only()
        .timeout(timeout)
        .build()
        .expect("Could not build client")
}

fn start_adaptors(configuration: &Configuration, client: &Client) -> Vec<DaikinAdaptor> {
    let interval = configuration.interval();
    let hosts = configuration.hosts();

    hosts
        .iter()
        .map(|host| {
            info!("Reading from Daikin adaptor {}", host);

            let daikin_adaptor = DaikinAdaptor::new(host.clone(), interval);

            let client = client.clone();
            let adaptor = daikin_adaptor.clone();

            tokio::spawn(async move {
                adaptor.read_loop(client).await;
            });

            daikin_adaptor
        })
        .collect()
}

#[tokio::main]
async fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let configuration = Configuration::load_from_next_arg();
    let client = new_client(configuration.timeout());

    let adaptors = start_adaptors(&configuration, &client);

    DaikinExporter::new(configuration.bind_address())
        .run(adaptors)
        .await;
}
