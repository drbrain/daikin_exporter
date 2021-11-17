use serde::Deserialize;

use std::fs;
use std::path::Path;

#[derive(Deserialize)]
pub struct Configuration {
    bind_address: Option<String>,
    hosts: Vec<String>,
    interval: Option<u64>,
    timeout: Option<u64>,
}

impl Configuration {
    pub fn load<P: AsRef<Path>>(path: P) -> Self {
        let source = fs::read_to_string(path).unwrap();

        toml::from_str(&source).unwrap()
    }

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

    pub fn bind_address(&self) -> String {
        self.bind_address
            .as_ref()
            .unwrap_or(&"0.0.0.0:9150".to_string())
            .to_string()
    }

    pub fn interval(&self) -> std::time::Duration {
        let interval = self.interval.unwrap_or(2000);

        std::time::Duration::from_millis(interval)
    }

    pub fn timeout(&self) -> std::time::Duration {
        let timeout = self.timeout.unwrap_or(250);

        std::time::Duration::from_millis(timeout)
    }
}
