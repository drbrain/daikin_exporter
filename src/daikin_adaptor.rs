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
        let power_on = basic_info.get("pow").unwrap().to_string();

        let control_info = match control_info(&client, &self.host).await {
            Ok(i) => i,
            Err(e) => {
                debug!("error {:?}", e);
                return;
            }
        };

        let set_temp = control_info.get("stemp").unwrap().to_string();
        let set_humid = control_info.get("shum").unwrap().to_string();
        let mode = control_info.get("mode").unwrap().to_string();
        let mut fan_rate = control_info.get("f_rate").unwrap().to_string();

        if fan_rate == "A" {
            fan_rate = "1".to_string();
        } else if fan_rate == "B" {
            fan_rate = "2".to_string();
        }

        let fan_dir = control_info.get("f_dir").unwrap().to_string();

        let sensor_info = match sensor_info(&client, &self.host).await {
            Ok(i) => i,
            Err(e) => {
                debug!("error {:?}", e);
                return;
            }
        };

        let unit_temp = sensor_info.get("htemp").unwrap().to_string();
        let outdoor_temp = sensor_info.get("otemp").unwrap().to_string();

        let mut info = self.info.lock().await;

        info.insert("device_name".to_string(), device_name);
        info.insert("power_on".to_string(), power_on);

        info.insert("mode".to_string(), mode);
        info.insert("set_temp".to_string(), set_temp);
        info.insert("set_humid".to_string(), set_humid);
        info.insert("fan_rate".to_string(), fan_rate);
        info.insert("fan_dir".to_string(), fan_dir);

        info.insert("unit_temp".to_string(), unit_temp);
        info.insert("outdoor_temp".to_string(), outdoor_temp);
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

async fn control_info(client: &Client, host: &str) -> DaikinResponse {
    debug!("Updating control info for {}", host);

    let url = format!("http://{}/aircon/get_control_info", host);

    let response = client.get(&url).send().await?;

    result_hash(response).await
}

async fn sensor_info(client: &Client, host: &str) -> DaikinResponse {
    debug!("Updating sensor info for {}", host);

    let url = format!("http://{}/aircon/get_sensor_info", host);

    let response = client.get(&url).send().await?;

    result_hash(response).await
}
