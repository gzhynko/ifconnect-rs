use std::borrow::Borrow;
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::io::Read;
use std::net::{UdpSocket, SocketAddr};
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use queues::{IsQueue, Queue};
use serde_json::Value;
use serde;
use serde::{Deserialize};
use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use crate::data::ConnectionData;
use crate::typed_value::TypedValue;

pub enum ConnectionState {
    Disconnected,
    Connected,
    Connecting,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct InstanceInformation {
    pub state: String,
    pub port: u32,
    #[serde(rename = "DeviceID")]
    pub device_id: String,
    pub aircraft: String,
    pub version: String,
    pub device_name: String,
    pub addresses: Vec<String>,
    pub livery: String
}

pub struct Connection {
    state: ConnectionState,
    connected_instance: Option<InstanceInformation>,
    udp_sock: Option<UdpSocket>,
    tcp_stream: Option<TcpStream>,
    data: ConnectionData,
    id_queue: Mutex<Queue<i32>>,
    bool_queue: Mutex<Queue<bool>>,
    value_queue: Mutex<Queue<TypedValue>>
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            connected_instance: None,
            udp_sock: None,
            tcp_stream: None,
            data: ConnectionData::new(),
            id_queue: Mutex::new(Queue::new()),
            bool_queue: Mutex::new(Queue::new()),
            value_queue: Mutex::new(Queue::new()),
        }
    }
}

impl Connection {
    /// Create a new Connection instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Discover IF instances over UDP.
    pub fn listen_udp(&mut self, udp_port: &u32, timeout_dur: Option<Duration>) -> Result<InstanceInformation, ()> {
        let addr: String = format!("0.0.0.0:{}", udp_port);
        let mut udp_sock = UdpSocket::bind(&addr).expect("failed to bind udp socket");

        udp_sock.set_read_timeout(timeout_dur).expect("failed to set read timeout on udp socket");

        // this will hang for the length of timeout_dur, then fail
        let mut buf = [0u8; 500];
        let mut string_result: &str = "";
        match udp_sock.recv(&mut buf) {
            Ok(_received) => {
                string_result = std::str::from_utf8(&buf).expect("failed to parse received udp message into string");
            },
            Err(err) => eprintln!("udp receive failed: {}", err)
        }
        if string_result.len() <= 0 { return Err(()) }

        // trim null characters and parse the json string
        string_result = string_result.trim_matches(char::from(0));
        println!("{}", string_result);
        let parsed_json: InstanceInformation = serde_json::from_str(string_result).expect("failed to parse received udp message into json");

        // keep the socket for reuse
        self.udp_sock = Some(udp_sock);

        self.connected_instance = Some(parsed_json.clone());
        Ok(parsed_json)
    }

    pub async fn start_tcp(&mut self, tcp_port: u32, tcp_address: String) -> Result<(), Box<dyn Error>> {
        let addr: String = format!("{}:{}", tcp_address, tcp_port).to_string();
        self.tcp_stream = Some(TcpStream::connect(addr).await.expect("failed to connect to tcp"));

        Ok(())
    }

    pub fn run_event_loop(&mut self) {
        tokio::spawn(async {
            self.update().await.unwrap();
        });
    }

    pub async fn update(&mut self) -> Result<(), Box<dyn Error>> {
        let tcp_stream = self.tcp_stream.as_ref().unwrap();

        let ready = tcp_stream.ready(Interest::READABLE | Interest::WRITABLE).await.unwrap();

        if ready.is_readable() {
            if self.data.read(&tcp_stream).is_err() {
                return Ok(())
            }
        }

        if ready.is_writable() {
            // ensure the queues can be locked, otherwise skip
            let ids_lock = self.id_queue.try_lock();
            let bool_lock = self.bool_queue.try_lock();
            let values_lock = self.value_queue.try_lock();
            if ids_lock.is_err() || bool_lock.is_err() || values_lock.is_err() { return Ok(()) }

            // write id
            let mut id_queue = ids_lock.unwrap();
            let next_id_entry = id_queue.remove();
            if !next_id_entry.is_err() {
                match tcp_stream.try_write(&next_id_entry.unwrap().to_be_bytes()) {
                    Ok(n) => {
                        println!("write {} id bytes", n);
                    }
                    // may still fail with `WouldBlock` if the readiness event is a false positive.
                    Err(ref e) if e.kind() == tokio::io::ErrorKind::WouldBlock => {
                        return Ok(())
                    }
                    Err(e) => {
                        return Err(e.into())
                    }
                }
            }

            // write bool
            let mut bool_queue = bool_lock.unwrap();
            let next_bool_entry = bool_queue.remove();
            if !next_bool_entry.is_err() {
                match tcp_stream.try_write(&(next_bool_entry.unwrap() as i32).to_be_bytes()) {
                    Ok(n) => {
                        println!("write {} bool bytes", n);
                    }
                    // may still fail with `WouldBlock` if the readiness event is a false positive.
                    Err(ref e) if e.kind() == tokio::io::ErrorKind::WouldBlock => {
                        return Ok(())
                    }
                    Err(e) => {
                        return Err(e.into())
                    }
                }
            }

            // write value
            let mut values_queue = values_lock.unwrap();
            let next_values_entry = values_queue.remove();
            if !next_values_entry.is_err() {
                match tcp_stream.try_write(&(next_values_entry.unwrap()).to_bytes_vec()) {
                    Ok(n) => {
                        println!("write {} value bytes", n);
                    }
                    // may still fail with `WouldBlock` if the readiness event is a false positive.
                    Err(ref e) if e.kind() == tokio::io::ErrorKind::WouldBlock => {
                        return Ok(())
                    }
                    Err(e) => {
                        return Err(e.into())
                    }
                }
            }
        }

        Ok(())
    }

    /// Awaits the id and bool queue locks and adds the new command id and false.
    pub async fn send_get_state(&self, command_id: i32) {
        let mut ids_lock = self.id_queue.lock().await;
        let mut bool_lock = self.bool_queue.lock().await;

        ids_lock.add(command_id);
        bool_lock.add(false);
    }
}
