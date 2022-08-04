use std::error::Error;
use std::net::{UdpSocket};
use std::sync::Arc;
use std::time::Duration;
use serde;
use serde::{Deserialize};
use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::{Mutex};
use crate::data::ConnectionData;
use crate::error::ManifestError;
use crate::event_args::{ReceivedDataArgs, ReceivedManifestArgs};
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
    tcp_stream: Option<Arc<Mutex<TcpStream>>>,
    pub data: ConnectionData,
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            connected_instance: None,
            udp_sock: None,
            tcp_stream: None,
            data: ConnectionData::new(),
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
        let udp_sock = UdpSocket::bind(&addr).expect("failed to bind udp socket");

        udp_sock.set_read_timeout(timeout_dur).expect("failed to set read timeout on udp socket");

        // this will block the thread for the length of timeout_dur, then fail if not received data
        let mut buf = [0u8; 500];
        let mut string_result: &str = "";
        match udp_sock.recv(&mut buf) {
            Ok(_received) => {
                string_result = std::str::from_utf8(&buf).expect("failed to parse received udp message into string");
            },
            Err(err) => eprintln!("udp receive failed: {}", err)
        }
        if string_result.is_empty() { return Err(()) }

        // trim null characters and parse the json string
        string_result = string_result.trim_matches(char::from(0));
        println!("{}", string_result);
        let parsed_instance: InstanceInformation = serde_json::from_str(string_result).expect("failed to parse received udp message into json");

        // keep the socket for reuse
        self.udp_sock = Some(udp_sock);

        self.connected_instance = Some(parsed_instance.clone());
        Ok(parsed_instance)
    }

    pub async fn start_tcp(&mut self, tcp_port: u32, tcp_address: String) -> Result<(), Box<dyn Error>> {
        let addr: String = format!("{}:{}", tcp_address, tcp_port);
        let stream = TcpStream::connect(addr).await.expect("failed to connect to tcp");
        self.tcp_stream = Some(Arc::new(Mutex::new(stream)));

        Ok(())
    }

    pub async fn update(&mut self) -> Result<(), Box<dyn Error>> {
        let tcp_stream = self.tcp_stream.as_ref().unwrap().lock().await;

        let ready = tcp_stream.ready(Interest::READABLE | Interest::WRITABLE).await.unwrap();

        if ready.is_readable() {
            //println!("readable");
            if self.data.read(&tcp_stream).await.is_ok() {
                return Ok(())
            }
        }

        if ready.is_writable() {
            //println!("writable");
            if self.data.send(&tcp_stream).await.is_ok() {
                return Ok(())
            }
        }

        Ok(())
    }

    pub async fn get_manifest(&self) {
        self.data.send_get_state(-1).await
    }

    pub async fn get(&self, state: String) -> Result<(), ManifestError> {
        let manifest = self.data.get_manifest()?;
        let entry = manifest.get_entry_by_path(&state)?;
        self.get_id(entry.id).await;

        Ok(())
    }

    pub async fn set(&self, state: String, value: TypedValue) -> Result<(), ManifestError> {
        let manifest = self.data.get_manifest()?;
        let entry = manifest.get_entry_by_path(&state)?;
        self.set_id(entry.id, value).await;

        Ok(())
    }

    pub async fn run(&self, command: String) -> Result<(), ManifestError> {
        let manifest = self.data.get_manifest()?;
        let entry = manifest.get_entry_by_path(&command)?;
        self.run_id(entry.id).await;

        Ok(())
    }

    pub async fn get_id(&self, state_id: i32) {
        self.data.send_get_state(state_id).await
    }

    pub async fn set_id(&self, state_id: i32, value: TypedValue) {
        self.data.send_set_state(state_id, value).await
    }

    pub async fn run_id(&self, command_id: i32) {
        self.data.send_get_state(command_id).await
    }

    pub fn on_receive_data<F: Fn(ReceivedDataArgs) + Send + 'static>(&mut self, func: F) {
        self.data.add_received_data_callback(func)
    }

    pub fn on_receive_manifest<F: Fn(ReceivedManifestArgs) + Send + 'static>(&mut self, func: F) {
        self.data.add_received_manifest_callback(func)
    }
}
