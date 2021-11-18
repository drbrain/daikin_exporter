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

macro_rules! set_metric {
    ($info: expr, $device_name:expr, $metric_name:expr) => {
        if let Some(value) = $info.get(stringify!($metric_name)) {
            match value.parse() {
                Ok(metric_value) => {
                    $metric_name
                        .with_label_values(&[&$device_name])
                        .set(metric_value);
                }
                Err(_) => (),
            }
        }
    };
}

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

    let power_on =
        register_gauge_vec!("daikin_power_on", "Daikin unit is on", &["device"]).unwrap();

    let mode = register_gauge_vec!(
        "daikin_mode",
        "Daikin mode (0, 1, 7 auto, 2 dehumidify, 3 cool, 4 heat, 6 fan)",
        &["device"]
    )
    .unwrap();

    let set_humid = register_gauge_vec!(
        "daikin_set_humidity_relative",
        "Humidity set-point",
        &["device"]
    )
    .unwrap();

    let set_temp = register_gauge_vec!(
        "daikin_set_temperature_degrees",
        "Temperature set-point",
        &["device"]
    )
    .unwrap();

    let fan_rate = register_gauge_vec!(
        "daikin_fan_rate",
        "Daikin fan rate (1 auto, 2 silence, 3–7 level 1–5)",
        &["device"]
    )
    .unwrap();

    let fan_dir = register_gauge_vec!(
        "daikin_fan_direction",
        "Daikin fan direction (0 stopped, 1 vertical, 2 horizontal, 3 both)",
        &["device"]
    )
    .unwrap();

    let unit_temp = register_gauge_vec!(
        "daikin_unit_temperature_degrees",
        "Unit temperature",
        &["device"]
    )
    .unwrap();

    let outdoor_temp = register_gauge_vec!(
        "daikin_outdoor_temperature_degrees",
        "Outdoor temperature",
        &["device"]
    )
    .unwrap();

    let daily_runtime =
        register_gauge_vec!("daikin_daily_runtime_minutes", "Daily runtime", &["device"]).unwrap();

    loop {
        let _guard = exporter.wait_request();
        debug!("Updating metrics");

        for adaptor in &adaptors {
            let info = adaptor.info.lock().await;

            let device_name = match info.get("device_name") {
                Some(name) => name,
                None => {
                    continue;
                }
            };

            set_metric!(info, device_name, power_on);
            set_metric!(info, device_name, mode);
            set_metric!(info, device_name, set_humid);
            set_metric!(info, device_name, set_temp);
            set_metric!(info, device_name, fan_rate);
            set_metric!(info, device_name, fan_dir);
            set_metric!(info, device_name, unit_temp);
            set_metric!(info, device_name, outdoor_temp);
            set_metric!(info, device_name, daily_runtime);

            debug!("Updated metrics for {} ({})", device_name, adaptor.host);
        }
    }
}