use anyhow::Context;
use anyhow::Result;

use crate::Configuration;

use lazy_static::lazy_static;

use nix::ifaddrs::getifaddrs;
use nix::sys::socket::SockAddr;

use prometheus::register_int_counter_vec;
use prometheus::IntCounterVec;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use log::trace;

use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::time::sleep;

type AddressSender = broadcast::Sender<String>;
type ErrorSender = mpsc::Sender<anyhow::Error>;

const DISCOVER_PORT: u16 = 30050;

lazy_static! {
    static ref REQUESTS: IntCounterVec = register_int_counter_vec!(
        "daikin_udp_discover_requests_total",
        "Number of UDP discover requests made to Daikin adaptors",
        &["address"],
    )
    .unwrap();
    static ref RESPONSES: IntCounterVec = register_int_counter_vec!(
        "daikin_udp_discover_responses_total",
        "Number of UDP discover responses read from Daikin adaptors",
        &["host"],
    )
    .unwrap();
}

// Discover daikin units on broadcast addresses

#[derive(Clone)]
pub struct DaikinDiscover {
    channel: AddressSender,
    socket: Arc<UdpSocket>,

    major_interval: Duration,
    minor_interval: Duration,
}

impl DaikinDiscover {
    pub async fn new(configuration: &Configuration) -> Result<Self> {
        let major_interval = configuration.discover_major_interval();
        let minor_interval = configuration.discover_minor_interval();

        let (channel, _) = broadcast::channel(16);

        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .context("Unable to start Daikin discovery")?;

        socket
            .set_broadcast(true)
            .context("Unable to start Daikin discovery")?;

        let socket = Arc::new(socket);

        Ok(DaikinDiscover {
            channel,
            socket,
            major_interval,
            minor_interval,
        })
    }

    pub async fn start(self, error_tx: ErrorSender) -> AddressSender {
        let this = self.clone();

        let broadcast_error_tx = error_tx.clone();

        tokio::spawn(async move {
            this.broadcast_loop(broadcast_error_tx).await;
        });

        let listen_error_tx = error_tx.clone();
        let this = self.clone();

        tokio::spawn(async move {
            this.listen_loop(listen_error_tx).await;
        });

        self.channel.clone()
    }

    pub async fn broadcast(&self, address: SocketAddr) -> Result<()> {
        trace!("Discovering for {}", address);

        self.socket
            .send_to(b"DAIKIN_UDP/common/basic_info", address)
            .await
            .with_context(|| format!("Unable to send discover request to {}", address))?;

        REQUESTS
            .with_label_values(&[&address.ip().to_string()])
            .inc();

        Ok(())
    }

    pub async fn broadcast_loop(&self, error_tx: ErrorSender) {
        loop {
            let addresses = match broadcast_addresses() {
                Ok(a) => a,
                Err(e) => {
                    error_tx.send(e).await.unwrap();
                    return;
                }
            };

            for address in &addresses {
                if let Err(e) = self.broadcast(address.clone()).await {
                    error_tx.send(e).await.unwrap();
                    return;
                };
            }

            sleep(self.minor_interval).await;

            for address in addresses {
                if let Err(e) = self.broadcast(address).await {
                    error_tx.send(e).await.unwrap();
                    return;
                };
            }

            sleep(self.major_interval).await;
        }
    }

    pub async fn listen(&self) -> Result<()> {
        loop {
            let mut buf = vec![0; 1000];

            let (n, a) = self
                .socket
                .recv_from(&mut buf)
                .await
                .context("Unable to read discover response")?;

            RESPONSES.with_label_values(&[&a.ip().to_string()]).inc();

            trace!(
                "received {} bytes {:?} from {}",
                n,
                String::from_utf8(buf[..n].to_vec()),
                a
            );

            let ip = a.ip().to_string();

            self.channel
                .send(ip.clone())
                .with_context(|| format!("Unable to notify of discovered unit IP {}", ip))?;
        }
    }

    pub async fn listen_loop(&self, error_tx: ErrorSender) {
        loop {
            if let Err(e) = self.listen().await {
                error_tx.send(e).await.unwrap();
                break;
            }
        }
    }
}

// Local broadcast addresses
fn broadcast_addresses() -> Result<Vec<SocketAddr>> {
    let ifaddrs = getifaddrs().context("Unable to find network interfaces")?;

    let broadcast_addresses = ifaddrs
        .into_iter()
        .filter(|ifaddr| ifaddr.broadcast.is_some())
        .map(|ifaddr| match ifaddr.broadcast.unwrap() {
            SockAddr::Inet(a) => a.ip(),
            other => unreachable!("unhandled broadcast address {:?}, nix bug?", other),
        })
        .map(|broadcast_addr| broadcast_addr.to_string().parse().unwrap())
        .map(|ipaddr| SocketAddr::new(ipaddr, DISCOVER_PORT))
        .collect();

    Ok(broadcast_addresses)
}
