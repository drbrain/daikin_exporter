use lazy_static::lazy_static;

use log::debug;
use log::error;
use log::trace;

use prometheus::core::Collector;
use prometheus::register_gauge_vec;
use prometheus::register_histogram_vec;
use prometheus::register_int_counter_vec;
use prometheus::register_int_gauge_vec;
use prometheus::GaugeVec;
use prometheus::HistogramVec;
use prometheus::IntCounterVec;
use prometheus::IntGaugeVec;

use reqwest::Client;

use std::collections::HashMap;
use std::time::Duration;

use tokio::time::interval;
use tokio::time::MissedTickBehavior;

type Info = HashMap<String, String>;
type DaikinResponse = Result<Info, reqwest::Error>;

macro_rules! set_metric {
    ( $metric:ident, $value:ident, $parse:ty, $device_name:ident) => {
        if let Ok(v) = $value.parse::<$parse>() {
            $metric.with_label_values(&[&$device_name]).set(v);
        } else {
            let desc = $metric.desc()[0];
            error!(
                "Invalid value {} for metric {} {} ({})",
                $value, $device_name, desc.fq_name, desc.help
            );
        }
    };
}

macro_rules! set_metric_tenth {
    ( $metric:ident, $value:ident, $parse:ty, $device_name:ident) => {
        if let Ok(v) = $value.parse::<$parse>() {
            $metric
                .with_label_values(&[&$device_name])
                .set(v / 10 as $parse);
        } else {
            let desc = $metric.desc()[0];
            error!(
                "Invalid value {} for metric {} {} ({})",
                $value, $device_name, desc.fq_name, desc.help
            );
        }
    };
}

lazy_static! {
    static ref REQUESTS: IntCounterVec = register_int_counter_vec!(
        "daikin_http_requests_total",
        "Number of HTTP requests made to Daikin adaptors",
        &["host", "path"],
    )
    .unwrap();
    static ref ERRORS: IntCounterVec = register_int_counter_vec!(
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
    static ref POWER_ON: IntGaugeVec =
        register_int_gauge_vec!("daikin_power_on", "Daikin unit is on", &["device"]).unwrap();
    static ref MODE: IntGaugeVec = register_int_gauge_vec!(
        "daikin_mode",
        "Daikin mode (0, 1, 7 auto, 2 dehumidify, 3 cool, 4 heat, 6 fan)",
        &["device"]
    )
    .unwrap();
    static ref SET_HUMID: IntGaugeVec = register_int_gauge_vec!(
        "daikin_set_humidity_relative",
        "Humidity set-point",
        &["device"]
    )
    .unwrap();
    static ref SET_TEMP: GaugeVec = register_gauge_vec!(
        "daikin_set_temperature_degrees",
        "Temperature set-point",
        &["device"]
    )
    .unwrap();
    static ref FAN_RATE: IntGaugeVec = register_int_gauge_vec!(
        "daikin_fan_rate",
        "Daikin fan rate (1 auto, 2 silence, 3–7 level 1–5)",
        &["device"]
    )
    .unwrap();
    static ref FAN_DIR: IntGaugeVec = register_int_gauge_vec!(
        "daikin_fan_direction",
        "Daikin fan direction (0 stopped, 1 vertical, 2 horizontal, 3 both)",
        &["device"]
    )
    .unwrap();
    static ref UNIT_TEMP: GaugeVec = register_gauge_vec!(
        "daikin_unit_temperature_degrees",
        "Unit temperature",
        &["device"]
    )
    .unwrap();
    static ref OUTDOOR_TEMP: GaugeVec = register_gauge_vec!(
        "daikin_outdoor_temperature_degrees",
        "Outdoor temperature",
        &["device"]
    )
    .unwrap();
    static ref COMPRESSOR_DEMAND: IntGaugeVec = register_int_gauge_vec!(
        "daikin_compressor_demand_percent",
        "Compressor demand (0–100)",
        &["device"]
    )
    .unwrap();
    static ref DAILY_RUNTIME: IntGaugeVec =
        register_int_gauge_vec!("daikin_daily_runtime_minutes", "Daily runtime", &["device"])
            .unwrap();
    static ref MONITOR_FAN_SPEED: IntGaugeVec = register_int_gauge_vec!(
        "daikin_monitor_fan_speed_percent",
        "Unit fan speed (0–100)",
        &["device"]
    )
    .unwrap();
    static ref MONITOR_RAWRTMP: IntGaugeVec = register_int_gauge_vec!(
        "daikin_monitor_rawr_temperature_degrees",
        "Room air temperature",
        &["device"]
    )
    .unwrap();
    static ref MONITOR_TRTMP: IntGaugeVec = register_int_gauge_vec!(
        "daikin_monitor_tr_temperature_degrees",
        "tr tempurature",
        &["device"]
    )
    .unwrap();
    static ref MONITOR_FANGL: IntGaugeVec =
        register_int_gauge_vec!("daikin_monitor_fangl", "fangl", &["device"]).unwrap();
    static ref MONITOR_HETMP: IntGaugeVec = register_int_gauge_vec!(
        "daikin_monitor_heat_exchanger_temperature_degrees",
        "Heat exchanger",
        &["device"]
    )
    .unwrap();
    static ref MONITOR_RESETS: IntGaugeVec = register_int_gauge_vec!(
        "daikin_monitor_reset_count",
        "Wifi adatptor resets",
        &["device"]
    )
    .unwrap();
    static ref MONITOR_ROUTER_DISCONNECTS: IntGaugeVec = register_int_gauge_vec!(
        "daikin_monitor_router_disconnect_count",
        "Router disconnections",
        &["device"]
    )
    .unwrap();
    static ref MONITOR_POLLING_ERRORS: IntGaugeVec = register_int_gauge_vec!(
        "daikin_monitor_polling_error_count",
        "Polling errors",
        &["device"]
    )
    .unwrap();
}

#[derive(Clone)]
pub struct DaikinAdaptor {
    pub host: String,
    interval: Duration,

    device_name: Option<String>,
}

impl DaikinAdaptor {
    pub fn new(host: String, interval: Duration) -> Self {
        let device_name = None;

        DaikinAdaptor {
            host,
            interval,
            device_name,
        }
    }

    pub async fn read_loop(&mut self, client: Client) {
        let mut interval = interval(self.interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            self.read_device(&client).await;
        }
    }

    async fn read_device(&mut self, client: &Client) {
        if let Some(basic_info) = self.get_info(client, "common/basic_info").await {
            let device_name = percent_decode(basic_info.get("name").unwrap());

            self.device_name = Some(device_name.clone());

            let power_on = basic_info.get("pow").unwrap().to_string();

            set_metric!(POWER_ON, power_on, i64, device_name);
        }

        let device_name = match &self.device_name {
            Some(name) => name,
            None => {
                // We haven't retrieved the device name yet so we won't be able to assign the
                // device label to any of the metrics we will collect below.
                return;
            }
        };

        if let Some(control_info) = self.get_info(client, "aircon/get_control_info").await {
            let set_temp = control_info.get("stemp").unwrap().to_string();
            let set_humid = control_info.get("shum").unwrap().to_string();
            let mode = control_info.get("mode").unwrap().to_string();
            let fan_rate = control_info.get("f_rate").unwrap().to_string();

            let fan_rate = if fan_rate == "A" {
                1
            } else if fan_rate == "B" {
                2
            } else {
                fan_rate.parse::<i64>().unwrap()
            };

            let fan_dir = control_info.get("f_dir").unwrap().to_string();

            set_metric!(MODE, mode, i64, device_name);
            set_metric!(SET_TEMP, set_temp, f64, device_name);
            set_metric!(SET_HUMID, set_humid, i64, device_name);
            FAN_RATE.with_label_values(&[device_name]).set(fan_rate);
            set_metric!(FAN_DIR, fan_dir, i64, device_name);
        }

        if let Some(sensor_info) = self.get_info(client, "aircon/get_sensor_info").await {
            let unit_temp = sensor_info.get("htemp").unwrap().to_string();
            let outdoor_temp = sensor_info.get("otemp").unwrap().to_string();
            let compressor_demand = sensor_info.get("cmpfreq").unwrap().to_string();

            set_metric!(UNIT_TEMP, unit_temp, f64, device_name);
            set_metric!(OUTDOOR_TEMP, outdoor_temp, f64, device_name);
            set_metric!(COMPRESSOR_DEMAND, compressor_demand, i64, device_name);
        }

        if let Some(week_power) = self.get_info(client, "aircon/get_week_power").await {
            let daily_runtime = week_power.get("today_runtime").unwrap().to_string();

            set_metric!(DAILY_RUNTIME, daily_runtime, i64, device_name);
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

            set_metric!(MONITOR_FAN_SPEED, monitor_fan_speed, i64, device_name);
            set_metric_tenth!(MONITOR_RAWRTMP, monitor_rawrtmp, i64, device_name);
            set_metric_tenth!(MONITOR_TRTMP, monitor_trtmp, i64, device_name);
            set_metric!(MONITOR_FANGL, monitor_fangl, i64, device_name);
            set_metric_tenth!(MONITOR_HETMP, monitor_hetmp, i64, device_name);
            set_metric!(MONITOR_RESETS, monitor_resets, i64, device_name);
            set_metric!(
                MONITOR_ROUTER_DISCONNECTS,
                monitor_router_disconnects,
                i64,
                device_name
            );
            set_metric!(
                MONITOR_POLLING_ERRORS,
                monitor_polling_errors,
                i64,
                device_name
            );
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

fn percent_decode(encoded: &str) -> String {
    let mut encoded = encoded.split('%');

    encoded.next(); // skip leading empty value

    let decoded = encoded
        .map(|code| u8::from_str_radix(code, 16).unwrap())
        .collect();

    String::from_utf8(decoded).unwrap()
}

// Decodes "4142" to "AB"

fn decode(encoded: &str) -> String {
    let pairs = encoded.len() / 2;
    let mut decoded = Vec::with_capacity(pairs);

    for pair in 0..pairs {
        let offset = pair * 2;
        decoded.push(u8::from_str_radix(&encoded[offset..offset + 2], 16).unwrap());
    }

    String::from_utf8(decoded).unwrap()
}

async fn result_hash(response: reqwest::Response) -> DaikinResponse {
    let url = response.url().clone();
    let body = response.text().await?;

    trace!("Request {} received: {}", url, body);

    let pairs = body.split(',');

    let mut result = HashMap::new();

    for pair in pairs {
        let mut entry = pair.split('=');

        let key = entry.next().unwrap().to_string();
        let value = entry.next().unwrap().to_string();

        result.insert(key, value);
    }

    Ok(result)
}
