mod configuration;
mod daikin_adaptor;

use configuration::Configuration;
use daikin_adaptor::DaikinAdaptor;

use env_logger::Builder;
use env_logger::Env;

use log::debug;
use log::info;

use prometheus_exporter::prometheus::register_gauge_vec;
use prometheus_exporter::Exporter;

use reqwest::Client;

use std::net::SocketAddr;
use std::time::Duration;

fn new_client(timeout: Duration) -> Client {
    Client::builder()
        .connect_timeout(timeout)
        .http1_only()
        .timeout(timeout)
        .build()
        .expect("Could not build client")
}

fn new_exporter(bind_address: String) -> Exporter {
    let addr: SocketAddr = bind_address
        .parse()
        .expect(&format!("can not parse listen address {}", bind_address));

    prometheus_exporter::start(addr).expect(&format!("can not start exporter on {}", bind_address))
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

    let exporter = new_exporter(configuration.bind_address());

    let set_point_degrees = register_gauge_vec!(
        "daikin_set_point_degrees",
        "Temperature set-point",
        &["device"]
    )
    .unwrap();

    loop {
        let _guard = exporter.wait_request();
        debug!("Updating metrics");

        for adaptor in &adaptors {
            let info = adaptor.info.lock().await;

            let device_name = info.get("device_name").unwrap();
            let set_point: f64 = info.get("set_point").unwrap().parse().unwrap();

            set_point_degrees
                .with_label_values(&[&device_name])
                .set(set_point);

            debug!("Updated metrics for {} ({})", device_name, adaptor.host);
        }
    }
}
