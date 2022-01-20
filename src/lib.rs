pub mod connection;
pub mod data;
pub mod manifest;
pub mod typed_value;
pub mod error;

use std::os::unix::net::SocketAddr;
use std::time::Duration;
use crate::connection::Connection;

pub const UDP_PORT: u32 = 15000;
pub const TCP_PORT_V2: u32 = 10112;

pub async fn connect(udp_port: u32, tcp_port: u32, udp_timeout_dur: Option<Duration>) {
}
