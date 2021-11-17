use env_logger::Builder;
use env_logger::Env;

use log::debug;

use prometheus_exporter::prometheus::register_gauge_vec;

use reqwest::Client;

use std::collections::HashMap;
use std::net::SocketAddr;

mod configuration;
use configuration::Configuration;

type DaikinResponse = Result<HashMap<String, String>, reqwest::Error>;

fn new_client(timeout: std::time::Duration) -> Client {
    Client::builder()
        .connect_timeout(timeout)
        .http1_only()
        .timeout(timeout)
        .build()
        .expect("Could not build client")
}

fn decode(encoded: &String) -> String {
    let mut encoded = encoded.split("%");

    encoded.next(); // skip leading empty value

    let decoded = encoded
        .map(|code| u8::from_str_radix(code, 16).unwrap())
        .collect();

    String::from_utf8(decoded).unwrap()
}

async fn result_hash(response: reqwest::Response) -> DaikinResponse {
    let body = response.text().await?;

    let pairs = body.split(",");

    let mut result = HashMap::new();

    for pair in pairs {
        let mut entry = pair.split("=");

        let key = entry.next().unwrap().to_string();
        let value = entry.next().unwrap().to_string();

        result.insert(key, value);
    }

    Ok(result)
}

async fn basic_info(client: &Client, addr: &str) -> DaikinResponse {
    let url = format!("http://{}/common/basic_info", addr);

    let response = client.get(&url).send().await?;

    result_hash(response).await
}

async fn get_control_info(client: &Client, addr: &str) -> DaikinResponse {
    let url = format!("http://{}/aircon/get_control_info", addr);

    let response = client.get(&url).send().await?;

    result_hash(response).await
}

#[tokio::main]
async fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let configuration = Configuration::load_from_next_arg();

    let addr_raw = configuration.bind_address();
    let addr: SocketAddr = addr_raw.parse().expect("can not parse listen addr");

    let exporter = prometheus_exporter::start(addr).expect("can not start exporter");

    let update_interval = configuration.interval();

    let set_point_degrees = register_gauge_vec!(
        "daikin_set_point_degrees",
        "Temperature set-point",
        &["device"]
    )
    .unwrap();

    let client = new_client(configuration.timeout());

    loop {
        let _guard = exporter.wait_duration(update_interval);

        debug!("Updating basic info");

        let basic_info = match basic_info(&client, "10.101.28.64").await {
            Ok(i) => i,
            Err(e) => {
                debug!("error {:?}", e);
                continue;
            }
        };

        let device_name = decode(basic_info.get("name").unwrap());

        debug!("Updating control info");

        let control_info = match get_control_info(&client, "10.101.28.64").await {
            Ok(i) => i,
            Err(e) => {
                debug!("error {:?}", e);
                continue;
            }
        };

        let set_point: f64 = control_info.get("stemp").unwrap().parse().unwrap();

        debug!("New set point: {}", set_point);

        set_point_degrees
            .with_label_values(&[&device_name])
            .set(set_point);
    }
}
