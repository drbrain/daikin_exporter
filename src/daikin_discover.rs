use anyhow::Context;
use anyhow::Result;

use crate::Configuration;

use lazy_static::lazy_static;

use nix::ifaddrs::getifaddrs;
use nix::sys::socket::SockAddr;

use prometheus_exporter::prometheus::register_counter_vec;
use prometheus_exporter::prometheus::CounterVec;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use log::trace;

use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tokio::time::sleep;
use tokio::time::timeout;

type AddressSender = broadcast::Sender<String>;
type ErrorSender = oneshot::Sender<anyhow::Error>;
type ErrorReceiver = oneshot::Receiver<anyhow::Error>;

const DISCOVER_PORT: u16 = 30050;

lazy_static! {
    static ref REQUESTS: CounterVec = register_counter_vec!(
        "daikin_udp_discover_requests_total",
        "Number of UDP discover requests made to Daikin adaptors",
        &["address"],
    )
    .unwrap();
    static ref RESPONSES: CounterVec = register_counter_vec!(
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

    interval: Duration,
    max_wait: Duration,
}

impl DaikinDiscover {
    pub async fn new(configuration: &Configuration) -> Result<Self> {
        let interval = configuration.discover_interval();
        let max_wait = configuration.discover_timeout();

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
            interval,
            max_wait,
        })
    }

    pub async fn start(self) -> (AddressSender, ErrorReceiver) {
        let (error_tx, error_rx) = oneshot::channel();
        let this = self.clone();

        tokio::spawn(async move {
            this.discover_loop(error_tx).await;
        });

        (self.channel.clone(), error_rx)
    }

    pub async fn discover_loop(&self, error_tx: ErrorSender) {
        let mut error: Option<anyhow::Error> = None;

        // The ComfortControl iOS app sends two discover packets back-to-back about 200–250ms
        // apart and repeats the discover process about once every two seconds.
        loop {
            let addresses = match broadcast_addresses() {
                Ok(a) => a,
                Err(e) => {
                    error = Some(e);
                    break;
                }
            };

            for address in addresses {
                trace!("Discovering for {}", address);

                if let Err(e) = self.discover(address).await {
                    error = Some(e);
                    break;
                }

                if error.is_some() {
                    break;
                }

                sleep(Duration::from_millis(200)).await;

                if let Err(e) = self.discover(address).await {
                    error = Some(e);
                    break;
                }
            }

            if error.is_some() {
                break;
            }

            sleep(self.interval).await;
        }

        if let Some(e) = error {
            error_tx.send(e).unwrap();
        }
    }

    // Discover daikin units on the network broadcast `address` and send their IP addresses to
    // `tx`.
    //
    // This will wait up to 50ms after the last discovered unit.
    pub async fn discover(&self, address: SocketAddr) -> Result<()> {
        self.socket
            .send_to(b"DAIKIN_UDP/common/basic_info", address)
            .await
            .with_context(|| format!("Unable to send discover request to {}", address))?;

        REQUESTS
            .with_label_values(&[&address.ip().to_string()])
            .inc();

        loop {
            let mut buf = vec![0; 1000];
            let (n, a) = match timeout(self.max_wait, self.socket.recv_from(&mut buf)).await {
                Ok(r) => r.with_context(|| {
                    format!("Unable to read discover response send to {}", address)
                })?,
                Err(_) => {
                    return Ok(());
                }
            };

            RESPONSES.with_label_values(&[&a.ip().to_string()]).inc();

            trace!(
                "received {} bytes {:?} from {}",
                n,
                String::from_utf8(buf[..n].to_vec()),
                a
            );

            self.channel
                .send(a.ip().to_string())
                .with_context(|| format!("Unable to notify of discovered unit IP {}", a))?;
        }
    }
}

// Return local broadcast addresses
fn broadcast_addresses() -> Result<Vec<SocketAddr>> {
    let ifaddrs = getifaddrs().context("Unable to find network interfaces")?;

    let broadcast_addresses = ifaddrs
        .into_iter()
        .filter(|ifaddr| ifaddr.broadcast.is_some())
        .map(|ifaddr| match ifaddr.broadcast.unwrap() {
            SockAddr::Inet(a) => a.ip(),
            other => unreachable!("unhandled broadcast address {:?}", other),
        })
        .map(|broadcast_addr| broadcast_addr.to_string().parse().unwrap())
        .map(|ipaddr| SocketAddr::new(ipaddr, DISCOVER_PORT))
        .collect();

    Ok(broadcast_addresses)
}
