use crate::platform::Platform;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_time::Timer;

const SERVER_PORT: &str = env!("SERVER_PORT");

pub async fn server_task(mut platform: Platform<'static>) {
    let controller = platform.get_controller();
    let network_stack = platform.get_network_stack();
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];
    let port = SERVER_PORT.parse().unwrap_or(9001u16);

    loop {
        if network_stack.is_link_up() {
            let mut socket = UdpSocket::new(
                network_stack,
                &mut rx_meta,
                &mut rx_buffer,
                &mut tx_meta,
                &mut tx_buffer,
            );

            log::info!("Listening on port {port}...");
            socket.bind(port).expect("bind to socket");

            loop {
                let (n, ep) = socket.recv_from(&mut buf).await.expect("read from socket");
                log::debug!("Received {n} bytes from {ep}.");
                if let Ok(s) = core::str::from_utf8(&buf[..n]) {
                    log::info!("Received (from {ep}): {s}");
                }
            }
        } else {
            log::info!("Waiting for network link...");
            Timer::after_secs(1u64).await;
        }
    }
}
