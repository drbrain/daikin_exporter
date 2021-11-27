use anyhow::Context;
use anyhow::Result;

use log::info;

use prometheus_hyper::Server;

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::Notify;

pub struct DaikinExporter {
    bind_address: SocketAddr,
    shutdown: Arc<Notify>,
}

impl DaikinExporter {
    pub fn new(bind_address: String) -> Result<Self> {
        let bind_address: SocketAddr = bind_address
            .parse()
            .with_context(|| format!("Can't parse listen address {}", bind_address))?;

        let shutdown = Arc::new(Notify::new());

        let exporter = DaikinExporter {
            bind_address,
            shutdown,
        };

        Ok(exporter)
    }

    async fn run(&self) {
        info!("Starting server");
        Server::run(
            Arc::new(prometheus::default_registry().clone()),
            self.bind_address,
            self.shutdown.notified(),
        )
        .await
        .unwrap();
    }

    pub async fn start(self) {
        tokio::spawn(async move {
            self.run().await;
        });
    }
}
