use lazy_static::lazy_static;

use log::debug;

use prometheus_exporter::prometheus::register_counter_vec;
use prometheus_exporter::prometheus::register_histogram_vec;
use prometheus_exporter::prometheus::CounterVec;
use prometheus_exporter::prometheus::HistogramVec;

use reqwest::Client;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::sleep;

type Info = HashMap<String, String>;
type DaikinResponse = Result<Info, reqwest::Error>;

lazy_static! {
    static ref REQUESTS: CounterVec = register_counter_vec!(
        "daikin_http_requests_total",
        "Number of HTTP requests made to Daikin adaptors",
        &["host", "path"],
    )
    .unwrap();
    static ref ERRORS: CounterVec = register_counter_vec!(
        "daikin_http_request_errors_total",
        "Number of HTTP request errors made to Daikin adaptors",
        &["host", "path", "error_type"],
    )
    .unwrap();
    static ref DURATIONS: HistogramVec = register_histogram_vec!(
        "daikin_http_request_duration_seconds",
        "HTTP request durations",
        &["host", "path"],
    )
    .unwrap();
}

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
        if let Some(basic_info) = self.get_info(client, "common/basic_info").await {
            let device_name = percent_decode(basic_info.get("name").unwrap());
            let power_on = basic_info.get("pow").unwrap().to_string();

            let mut info = self.info.lock().await;

            info.insert("DEVICE_NAME".to_string(), device_name);
            info.insert("POWER_ON".to_string(), power_on);
        }

        {
            let info = self.info.lock().await;

            if !info.contains_key("DEVICE_NAME") {
                // We haven't retrieved the device name yet so we won't be able to assign the device
                // label to any of the metrics we will collect below.
                return;
            }
        }

        if let Some(control_info) = self.get_info(client, "aircon/get_control_info").await {
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

            let mut info = self.info.lock().await;

            info.insert("MODE".to_string(), mode);
            info.insert("SET_TEMP".to_string(), set_temp);
            info.insert("SET_HUMID".to_string(), set_humid);
            info.insert("FAN_RATE".to_string(), fan_rate);
            info.insert("FAN_DIR".to_string(), fan_dir);
        }

        if let Some(sensor_info) = self.get_info(client, "aircon/get_sensor_info").await {
            let unit_temp = sensor_info.get("htemp").unwrap().to_string();
            let outdoor_temp = sensor_info.get("otemp").unwrap().to_string();
            let compressor_demand = sensor_info.get("cmpfreq").unwrap().to_string();

            let mut info = self.info.lock().await;

            info.insert("UNIT_TEMP".to_string(), unit_temp);
            info.insert("OUTDOOR_TEMP".to_string(), outdoor_temp);
            info.insert("COMPRESSOR_DEMAND".to_string(), compressor_demand);
        }

        if let Some(week_power) = self.get_info(client, "aircon/get_week_power").await {
            let daily_runtime = week_power.get("today_runtime").unwrap().to_string();

            let mut info = self.info.lock().await;

            info.insert("DAILY_RUNTIME".to_string(), daily_runtime);
        }

        if let Some(monitor_data) = self.get_info(client, "aircon/get_monitordata").await {
            //let monitor_tap = decode(monitor_data.get("tap").unwrap());

            // Probably duplicate from control info
            //let monitor_mode = decode(monitor_data.get("mode").unwrap());

            // Probably duplicate from control info
            //let monitor_pow = decode(monitor_data.get("pow").unwrap());

            let monitor_fan_speed = decode(monitor_data.get("fan").unwrap());
            let monitor_rawrtmp = decode(monitor_data.get("rawrtmp").unwrap());
            let monitor_trtmp = decode(monitor_data.get("trtmp").unwrap());
            let monitor_fangl = decode(monitor_data.get("fangl").unwrap());
            let monitor_hetmp = decode(monitor_data.get("hetmp").unwrap());
            let monitor_resets = monitor_data.get("ResetCount").unwrap().to_string();
            let monitor_router_disconnects =
                monitor_data.get("RouterDisconCnt").unwrap().to_string();
            let monitor_polling_errors = monitor_data.get("PollingErrCnt").unwrap().to_string();

            let mut info = self.info.lock().await;

            info.insert("MONITOR_FAN_SPEED".to_string(), monitor_fan_speed);
            info.insert("MONITOR_RAWRTMP".to_string(), monitor_rawrtmp);
            info.insert("MONITOR_TRTMP".to_string(), monitor_trtmp);
            info.insert("MONITOR_FANGL".to_string(), monitor_fangl);
            info.insert("MONITOR_HETMP".to_string(), monitor_hetmp);
            info.insert("MONITOR_RESETS".to_string(), monitor_resets);
            info.insert(
                "MONITOR_ROUTER_DISCONNECTS".to_string(),
                monitor_router_disconnects,
            );
            info.insert("MONITOR_POLLING_ERRORS".to_string(), monitor_polling_errors);
        }
    }

    async fn get_info(&self, client: &Client, path: &str) -> Option<Info> {
        let path = path.to_string();
        let url = format!("http://{}/{}", self.host, path);

        debug!("Fetching {}", url);
        REQUESTS.with_label_values(&[&self.host, &path]).inc();
        let timer = DURATIONS
            .with_label_values(&[&self.host, &path])
            .start_timer();

        let response = client.get(&url).send().await;

        timer.observe_duration();

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                debug!("request error: {:?}", e);
                ERRORS
                    .with_label_values(&[&self.host, &path, "request"])
                    .inc();
                return None;
            }
        };

        match result_hash(response).await {
            Ok(r) => Some(r),
            Err(e) => {
                debug!("request body error: {:?}", e);
                ERRORS.with_label_values(&[&self.host, &path, "body"]).inc();
                None
            }
        }
    }
}

// Decodes "%41%42" to "AB"

fn percent_decode(encoded: &String) -> String {
    let mut encoded = encoded.split("%");

    encoded.next(); // skip leading empty value

    let decoded = encoded
        .map(|code| u8::from_str_radix(code, 16).unwrap())
        .collect();

    String::from_utf8(decoded).unwrap()
}

// Decodes "4142" to "AB"

fn decode(encoded: &String) -> String {
    let pairs = encoded.len() / 2;
    let mut decoded = Vec::with_capacity(pairs);

    for pair in 0..pairs {
        let offset = pair * 2;
        decoded.push(u8::from_str_radix(&encoded[offset..offset + 2], 16).unwrap());
    }

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
