pub mod connection;
pub mod data;
pub mod manifest;
pub mod typed_value;
pub mod error;
pub mod event_args;

use std::time::Duration;

pub const UDP_PORT: u32 = 15000;
pub const TCP_PORT_V2: u32 = 10112;
