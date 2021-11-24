use serde::Deserialize;

use std::fs;
use std::path::Path;

#[derive(Deserialize)]
pub struct Configuration {
    bind_address: Option<String>,
    hosts: Option<Vec<String>>,
    discover_interval: Option<u64>,
    discover_timeout: Option<u64>,
    refresh_interval: Option<u64>,
    refresh_timeout: Option<u64>,
}

impl Configuration {
    // Load a configuration file from `path`.
    pub fn load<P: AsRef<Path>>(path: P) -> Self {
        let source = fs::read_to_string(path).unwrap();

        toml::from_str(&source).unwrap()
    }

    // Load configuration from the next argument in the environment.
    /// TODO return anyhow::Result<Self> and exit from the caller
    pub fn load_from_next_arg() -> Self {
        let file = match std::env::args().nth(1) {
            None => {
                eprintln!("You must provide a configuration file");
                std::process::exit(1);
            }
            Some(f) => f,
        };

        Configuration::load(file)
    }

    // Bind address for Prometheus metric server
    pub fn bind_address(&self) -> String {
        self.bind_address
            .as_ref()
            .unwrap_or(&"0.0.0.0:9150".to_string())
            .to_string()
    }

    // Interval between discover requests.  Defaults to 5 minutes
    pub fn discover_interval(&self) -> std::time::Duration {
        let interval = self.discover_interval.unwrap_or(300_000);

        std::time::Duration::from_millis(interval)
    }

    /// TODO decouple discover broadcast and receive so we can listen forever and broadcast
    /// occasionally.  Then I can remove this timeout.
    pub fn discover_timeout(&self) -> std::time::Duration {
        let timeout = self.discover_timeout.unwrap_or(50);

        std::time::Duration::from_millis(timeout)
    }

    // Interval between HVAC unit data refreshes.  Defaults to 2 seconds.
    pub fn refresh_interval(&self) -> std::time::Duration {
        let interval = self.refresh_interval.unwrap_or(2000);

        std::time::Duration::from_millis(interval)
    }

    // Timeout to wait for an HVAC unit to respond.  Defaults to 250ms.
    pub fn refresh_timeout(&self) -> std::time::Duration {
        let timeout = self.refresh_timeout.unwrap_or(250);

        std::time::Duration::from_millis(timeout)
    }

    // Manually configured hosts.  Set this if UDP discovery is unreliable and you have given all
    // HVAC units static IPs.
    pub fn hosts(&self) -> Option<Vec<String>> {
        self.hosts.clone()
    }
}
