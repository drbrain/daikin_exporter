use crate::configuration::Configuration;
use crate::daikin_adaptor::DaikinAdaptor;

use log::info;

use reqwest::Client;

use std::collections::HashMap;
use std::time::Duration;

type Adaptors = HashMap<String, DaikinAdaptor>;

pub struct DaikinWatcher {
    adaptors: Adaptors,
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

        let adaptors = HashMap::new();

        DaikinWatcher {
            adaptors,
            client,
            hosts,
            interval,
        }
    }

    pub fn adaptors(&self) -> Vec<DaikinAdaptor> {
        self.adaptors.values().map(|a| a.clone()).collect()
    }

    pub fn start(&mut self) {
        for host in self.hosts.clone() {
            let adaptor = self.start_adaptor(&host);

            self.adaptors.insert(host, adaptor);
        }
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
