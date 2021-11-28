use crate::configuration::Configuration;
use crate::daikin_adaptor::DaikinAdaptor;

use log::info;

use reqwest::Client;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio::sync::Mutex;

type Adaptors = HashMap<String, DaikinAdaptor>;
type AddressSender = broadcast::Sender<String>;

#[derive(Clone)]
pub struct DaikinWatcher {
    adaptors: Arc<Mutex<Adaptors>>,
    discover: AddressSender,
    client: Client,
    hosts: Option<Vec<String>>,
    interval: Duration,
}

impl DaikinWatcher {
    pub fn new(discover: AddressSender, configuration: &Configuration) -> Self {
        let hosts = configuration.hosts();
        let interval = configuration.refresh_interval();
        let timeout = configuration.refresh_timeout();

        let client = Client::builder()
            .connect_timeout(timeout)
            .http1_only()
            .timeout(timeout)
            .build()
            .expect("Could not build client");

        let adaptors = Arc::new(Mutex::new(HashMap::new()));

        DaikinWatcher {
            adaptors,
            discover,
            client,
            hosts,
            interval,
        }
    }

    pub async fn start(&mut self) {
        if let Some(hosts) = self.hosts.clone() {
            for host in hosts {
                self.start_adaptor(&host).await;
            }
        }

        let mut discovered = self.discover.subscribe();
        let this = self.clone();

        tokio::spawn(async move {
            loop {
                let address = discovered.recv().await.unwrap();

                this.start_adaptor(&address).await;
            }
        });
    }

    async fn start_adaptor(&self, host: &str) {
        let mut adaptors = self.adaptors.lock().await;

        if adaptors.contains_key(host) {
            return;
        }

        info!("Watching Daikin adaptor {}", host);

        let daikin_adaptor = DaikinAdaptor::new(host.to_string(), self.interval);

        let client = self.client.clone();
        let mut adaptor = daikin_adaptor.clone();

        tokio::spawn(async move {
            adaptor.read_loop(client).await;
        });

        adaptors.insert(host.to_string(), daikin_adaptor);
    }
}
