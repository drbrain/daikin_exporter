use crate::configuration::Configuration;
use crate::daikin_adaptor::DaikinAdaptor;

use log::info;

use reqwest::Client;

use std::time::Duration;

pub struct DaikinWatcher {
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

        DaikinWatcher {
            client,
            hosts,
            interval,
        }
    }

    pub fn start(&self) -> Vec<DaikinAdaptor> {
        self.hosts
            .iter()
            .map(|host| {
                info!("Reading from Daikin adaptor {}", host);

                let daikin_adaptor = DaikinAdaptor::new(host.clone(), self.interval);

                let client = self.client.clone();
                let adaptor = daikin_adaptor.clone();

                tokio::spawn(async move {
                    adaptor.read_loop(client).await;
                });

                daikin_adaptor
            })
            .collect()
    }
}
