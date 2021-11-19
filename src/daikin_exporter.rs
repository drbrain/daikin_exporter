use crate::daikin_watcher::DaikinWatcher;

use log::debug;

use prometheus_exporter::prometheus::register_gauge_vec;
use prometheus_exporter::Exporter;

use std::net::SocketAddr;

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

macro_rules! set_metric_divide {
    ($info: expr, $device_name:expr, $metric_name:expr, $divisor:expr) => {
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

        let compressor_demand = register_gauge_vec!(
            "daikin_compressor_demand_percent",
            "Compressor demand (0–100)",
            &["device"]
        )
        .unwrap();

        let daily_runtime =
            register_gauge_vec!("daikin_daily_runtime_minutes", "Daily runtime", &["device"])
                .unwrap();

        let monitor_fan_speed = register_gauge_vec!(
            "daikin_monitor_fan_speed_percent",
            "Unit fan speed (0–100)",
            &["device"]
        )
        .unwrap();
        let monitor_rawrtmp = register_gauge_vec!(
            "daikin_monitor_rawr_temperature_degrees",
            "Room air temperature",
            &["device"]
        )
        .unwrap();
        let monitor_trtmp = register_gauge_vec!(
            "daikin_monitor_tr_temperature_degrees",
            "tr tempurature",
            &["device"]
        )
        .unwrap();
        let monitor_fangl =
            register_gauge_vec!("daikin_monitor_fangl", "fangl", &["device"]).unwrap();
        let monitor_hetmp = register_gauge_vec!(
            "daikin_monitor_heat_exchanger_temperature_degrees",
            "Heat exchanger",
            &["device"]
        )
        .unwrap();
        let monitor_resets = register_gauge_vec!(
            "daikin_monitor_reset_count",
            "Wifi adatptor resets",
            &["device"]
        )
        .unwrap();
        let monitor_router_disconnects = register_gauge_vec!(
            "daikin_monitor_router_disconnect_count",
            "Router disconnections",
            &["device"]
        )
        .unwrap();
        let monitor_polling_errors = register_gauge_vec!(
            "daikin_monitor_polling_error_count",
            "Polling errors",
            &["device"]
        )
        .unwrap();

        loop {
            let _guard = self.exporter.wait_request();

            debug!("Updating metrics");

            for adaptor in &watcher.adaptors() {
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
                set_metric!(info, device_name, compressor_demand);

                set_metric!(info, device_name, monitor_fan_speed);
                set_metric!(info, device_name, monitor_fangl);
                set_metric_divide!(info, device_name, monitor_hetmp, 10.0);
                set_metric!(info, device_name, monitor_polling_errors);
                set_metric_divide!(info, device_name, monitor_rawrtmp, 10.0);
                set_metric!(info, device_name, monitor_resets);
                set_metric!(info, device_name, monitor_router_disconnects);
                set_metric_divide!(info, device_name, monitor_trtmp, 10.0);

                debug!("Updated metrics for {} ({})", device_name, adaptor.host);
            }
        }
    }
}
