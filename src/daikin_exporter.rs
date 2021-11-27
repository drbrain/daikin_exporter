use anyhow::Context;
use anyhow::Result;

use log::info;

use prometheus_hyper::Server;

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::sync::Notify;

type ErrorSender = mpsc::Sender<anyhow::Error>;

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

    async fn run(&self, error_tx: ErrorSender) {
        info!("Starting server on {}", self.bind_address);

        let result = Server::run(
            Arc::new(prometheus::default_registry().clone()),
            self.bind_address,
            self.shutdown.notified(),
        )
        .await
        .with_context(|| format!("Failed to start server on {}", self.bind_address));

        if let Err(e) = result {
            error_tx
                .send(e)
                .await
                .expect("Error channel failed unexpectedly, bug?");
        }
    }

    pub async fn start(self, error_tx: ErrorSender) {
        tokio::spawn(async move {
            self.run(error_tx).await;
        });
    }
}
