use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use log::debug;

use reqwest::Client;

use tokio::sync::Mutex;
use tokio::time::sleep;

type DaikinResponse = Result<HashMap<String, String>, reqwest::Error>;
type Info = HashMap<String, String>;

#[derive(Clone)]
pub struct DaikinAdaptor {
    pub host: String,
    interval: Duration,
    pub info: Arc<Mutex<Info>>,
}

impl DaikinAdaptor {
    pub fn new(host: String, interval: Duration) -> Self {
        let info = Arc::new(Mutex::new(HashMap::new()));

        DaikinAdaptor {
            host,
            interval,
            info,
        }
    }

    pub async fn read_loop(&self, client: Client) {
        loop {
            sleep(self.interval).await;

            self.read_device(&client).await;
        }
    }

    async fn read_device(&self, client: &Client) {
        let basic_info = match basic_info(&client, &self.host).await {
            Ok(i) => i,
            Err(e) => {
                debug!("error {:?}", e);
                return;
            }
        };

        let device_name = decode(basic_info.get("name").unwrap());

        let control_info = match get_control_info(&client, &self.host).await {
            Ok(i) => i,
            Err(e) => {
                debug!("error {:?}", e);
                return;
            }
        };

        let set_point = control_info.get("stemp").unwrap().to_string();

        let mut info = self.info.lock().await;

        info.insert("device_name".to_string(), device_name);
        info.insert("set_point".to_string(), set_point);
    }
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

async fn basic_info(client: &Client, host: &str) -> DaikinResponse {
    debug!("Updating basic info for {}", host);

    let url = format!("http://{}/common/basic_info", host);

    let response = client.get(&url).send().await?;

    result_hash(response).await
}

async fn get_control_info(client: &Client, host: &str) -> DaikinResponse {
    debug!("Updating control info for {}", host);

    let url = format!("http://{}/aircon/get_control_info", host);

    let response = client.get(&url).send().await?;

    result_hash(response).await
}
