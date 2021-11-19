use crate::configuration::Configuration;
use crate::daikin_adaptor::DaikinAdaptor;

use log::info;

use reqwest::Client;

use std::time::Duration;

pub struct DaikinWatcher {
    adaptors: Vec<DaikinAdaptor>,
    client: Client,
    hosts: Vec<String>,
    interval: Duration,
}

impl DaikinWatcher {
    pub fn new(configuration: &Configuration) -> Self {
        let hosts = configuration.hosts();
        let interval = configuration.interval();
        let timeout = configuration.timeout();

        let client = Client::builder()
            .connect_timeout(timeout)
            .http1_only()
            .timeout(timeout)
            .build()
            .expect("Could not build client");

        let adaptors = Vec::new();

        DaikinWatcher {
            adaptors,
            client,
            hosts,
            interval,
        }
    }

    pub fn adaptors(&self) -> Vec<DaikinAdaptor> {
        self.adaptors.clone()
    }

    pub fn start(&mut self) {
        self.adaptors = self
            .hosts
            .iter()
            .map(|host| self.start_adaptor(host))
            .collect()
    }

    fn start_adaptor(&self, host: &String) -> DaikinAdaptor {
        info!("Watching Daikin adaptor {}", host);

        let daikin_adaptor = DaikinAdaptor::new(host.clone(), self.interval);

        let client = self.client.clone();
        let adaptor = daikin_adaptor.clone();

        tokio::spawn(async move {
            adaptor.read_loop(client).await;
        });

        daikin_adaptor
    }
}
