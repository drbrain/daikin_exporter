use crate::daikin_watcher::DaikinWatcher;

use lazy_static::lazy_static;

use log::debug;

use prometheus_exporter::prometheus::register_gauge_vec;
use prometheus_exporter::prometheus::GaugeVec;
use prometheus_exporter::Exporter;

use std::net::SocketAddr;

macro_rules! set_metric {
    ($info: expr, $device_name:expr, $metric_name:ident) => {
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

macro_rules! set_metric_divide {
    ($info: expr, $device_name:expr, $metric_name:ident, $divisor:expr) => {
        if let Some(value) = $info.get(stringify!($metric_name)) {
            match value.parse::<f64>() {
                Ok(metric_value) => {
                    $metric_name
                        .with_label_values(&[&$device_name])
                        .set(metric_value / $divisor);
                }
                Err(_) => (),
            }
        }
    };
}

lazy_static! {
    static ref POWER_ON: GaugeVec =
        register_gauge_vec!("daikin_power_on", "Daikin unit is on", &["device"]).unwrap();
    static ref MODE: GaugeVec = register_gauge_vec!(
        "daikin_mode",
        "Daikin mode (0, 1, 7 auto, 2 dehumidify, 3 cool, 4 heat, 6 fan)",
        &["device"]
    )
    .unwrap();
    static ref SET_HUMID: GaugeVec = register_gauge_vec!(
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
    static ref FAN_RATE: GaugeVec = register_gauge_vec!(
        "daikin_fan_rate",
        "Daikin fan rate (1 auto, 2 silence, 3–7 level 1–5)",
        &["device"]
    )
    .unwrap();
    static ref FAN_DIR: GaugeVec = register_gauge_vec!(
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
    static ref COMPRESSOR_DEMAND: GaugeVec = register_gauge_vec!(
        "daikin_compressor_demand_percent",
        "Compressor demand (0–100)",
        &["device"]
    )
    .unwrap();
    static ref DAILY_RUNTIME: GaugeVec =
        register_gauge_vec!("daikin_daily_runtime_minutes", "Daily runtime", &["device"]).unwrap();
    static ref MONITOR_FAN_SPEED: GaugeVec = register_gauge_vec!(
        "daikin_monitor_fan_speed_percent",
        "Unit fan speed (0–100)",
        &["device"]
    )
    .unwrap();
    static ref MONITOR_RAWRTMP: GaugeVec = register_gauge_vec!(
        "daikin_monitor_rawr_temperature_degrees",
        "Room air temperature",
        &["device"]
    )
    .unwrap();
    static ref MONITOR_TRTMP: GaugeVec = register_gauge_vec!(
        "daikin_monitor_tr_temperature_degrees",
        "tr tempurature",
        &["device"]
    )
    .unwrap();
    static ref MONITOR_FANGL: GaugeVec =
        register_gauge_vec!("daikin_monitor_fangl", "fangl", &["device"]).unwrap();
    static ref MONITOR_HETMP: GaugeVec = register_gauge_vec!(
        "daikin_monitor_heat_exchanger_temperature_degrees",
        "Heat exchanger",
        &["device"]
    )
    .unwrap();
    static ref MONITOR_RESETS: GaugeVec = register_gauge_vec!(
        "daikin_monitor_reset_count",
        "Wifi adatptor resets",
        &["device"]
    )
    .unwrap();
    static ref MONITOR_ROUTER_DISCONNECTS: GaugeVec = register_gauge_vec!(
        "daikin_monitor_router_disconnect_count",
        "Router disconnections",
        &["device"]
    )
    .unwrap();
    static ref MONITOR_POLLING_ERRORS: GaugeVec = register_gauge_vec!(
        "daikin_monitor_polling_error_count",
        "Polling errors",
        &["device"]
    )
    .unwrap();
}

pub struct DaikinExporter {
    exporter: Exporter,
}

impl DaikinExporter {
    pub fn new(bind_address: String) -> Self {
        let addr: SocketAddr = bind_address
            .parse()
            .expect(&format!("can not parse listen address {}", bind_address));

        let exporter = prometheus_exporter::start(addr)
            .expect(&format!("can not start exporter on {}", bind_address));

        DaikinExporter { exporter }
    }

    pub async fn run(&self, watcher: DaikinWatcher) {
        loop {
            let _guard = self.exporter.wait_request();

            debug!("Updating metrics");

            for adaptor in &watcher.adaptors().await {
                let info = adaptor.info.lock().await;

                let device_name = match info.get("DEVICE_NAME") {
                    Some(name) => name,
                    None => {
                        continue;
                    }
                };

                set_metric!(info, device_name, POWER_ON);
                set_metric!(info, device_name, MODE);
                set_metric!(info, device_name, SET_HUMID);
                set_metric!(info, device_name, SET_TEMP);
                set_metric!(info, device_name, FAN_RATE);
                set_metric!(info, device_name, FAN_DIR);
                set_metric!(info, device_name, UNIT_TEMP);
                set_metric!(info, device_name, OUTDOOR_TEMP);
                set_metric!(info, device_name, DAILY_RUNTIME);
                set_metric!(info, device_name, COMPRESSOR_DEMAND);

                set_metric!(info, device_name, MONITOR_FAN_SPEED);
                set_metric!(info, device_name, MONITOR_FANGL);
                set_metric_divide!(info, device_name, MONITOR_HETMP, 10.0);
                set_metric!(info, device_name, MONITOR_POLLING_ERRORS);
                set_metric_divide!(info, device_name, MONITOR_RAWRTMP, 10.0);
                set_metric!(info, device_name, MONITOR_RESETS);
                set_metric!(info, device_name, MONITOR_ROUTER_DISCONNECTS);
                set_metric_divide!(info, device_name, MONITOR_TRTMP, 10.0);

                debug!("Updated metrics for {} ({})", device_name, adaptor.host);
            }
        }
    }
}
