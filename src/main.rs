use env_logger::Builder;
use env_logger::Env;

use log::debug;

use prometheus_exporter::prometheus::register_gauge;
use prometheus_exporter::prometheus::Gauge;

use reqwest::Client;

use std::collections::HashMap;
use std::net::SocketAddr;

fn new_client() -> Client {
    let timeout = std::time::Duration::from_millis(100);

    Client::builder()
        .connect_timeout(timeout)
        .http1_only()
        .timeout(timeout)
        .build()
        .expect("Could not build client")
}

fn new_gauge(name: &str, description: &str) -> Gauge {
    register_gauge!(name, description).expect(&format!("Could not create gauge {}", name))
}

async fn result_hash(
    response: reqwest::Response,
) -> Result<HashMap<String, String>, reqwest::Error> {
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

async fn get_control_info(
    client: &Client,
    addr: &str,
) -> Result<HashMap<String, String>, reqwest::Error> {
    let url = format!("http://{}/aircon/get_control_info", addr);

    let response = client.get(&url).send().await?;

    result_hash(response).await
}

#[tokio::main]
async fn main() {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let addr_raw = "0.0.0.0:9150";
    let addr: SocketAddr = addr_raw.parse().expect("can not parse listen addr");

    let exporter = prometheus_exporter::start(addr).expect("can not start exporter");
    let update_interval = std::time::Duration::from_millis(1000);

    let set_point_degrees = new_gauge("daikin_set_point_degrees", "Temperature set-point");

    let client = new_client();

    loop {
        let _guard = exporter.wait_duration(update_interval);

        debug!("Updating metrics");

        let values = match get_control_info(&client, "10.101.28.64").await {
            Ok(v) => v,
            _ => {
                continue;
            }
        };

        let set_point: f64 = values.get("stemp").unwrap().parse().unwrap();

        debug!("New set point: {}", set_point);

        set_point_degrees.set(set_point);
    }
}
