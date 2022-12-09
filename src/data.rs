use std::convert::{TryInto};
use queues::{IsQueue, Queue};
use tokio::io;
use tokio::io::{Error};
use tokio::net::TcpStream;
use crate::manifest::Manifest;
use crate::typed_value::{Type, TypedValue};
use tokio::sync::{Mutex};
use crate::error::ManifestError;
use crate::event_args::{ReceivedDataArgs, ReceivedManifestArgs};


pub enum ChunkType {
    CommandId,
    DataLength,
    StringLength,
    Data
}

/// Contains methods for dealing with the data received from / sent to IF
pub struct ConnectionData {
    manifest: Option<Manifest>,

    // helper fields for reading data from the API
    expected_responses: Vec<i32>,
    next_chunk_type: ChunkType,
    current_id: i32,
    current_data_length: i32,
    current_string_len: i32,
    current_data_string: String,

    // queues for sending data to the API
    id_queue: Mutex<Queue<i32>>,
    bool_queue: Mutex<Queue<bool>>,
    value_queue: Mutex<Queue<TypedValue>>,

    // event callbacks
    data_received_callback: Option<Box<dyn Fn(ReceivedDataArgs) + Send>>,
    manifest_received_callback: Option<Box<dyn Fn(ReceivedManifestArgs) + Send>>,
}

impl Default for ConnectionData {
    fn default() -> Self {
        Self {
            manifest: None,

            expected_responses: Vec::new(),
            next_chunk_type: ChunkType::CommandId,
            current_id: 0,
            current_data_length: 0,
            current_string_len: 0,
            current_data_string: String::new(),

            id_queue: Mutex::new(Queue::new()),
            bool_queue: Mutex::new(Queue::new()),
            value_queue: Mutex::new(Queue::new()),

            data_received_callback: None,
            manifest_received_callback: None,
        }
    }
}

impl ConnectionData {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn read(&mut self, tcp_stream: &TcpStream) -> Result<(), Error> {
        let mut buf;
        // determine buffer size depending on the next expected chunk
        match self.next_chunk_type {
            ChunkType::CommandId => {
                buf = vec![0; 4];
            },
            ChunkType::DataLength => {
                buf = vec![0; 4];
            },
            ChunkType::StringLength => {
                buf = vec![0; 4];
            },
            ChunkType::Data => {
                buf = vec![0; 1024];
            },
        }

        return match tcp_stream.try_read(&mut buf) {
            Ok(len) => {
                // abort if there are no bytes
                if len == 0 { return Ok(()) }

                self.read_chunk(buf, len);

                Ok(())
            },
            // may still fail with `WouldBlock` if the readiness event is a false positive.
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                Err(e)
            },
            Err(e) => {
                Err(e)
            },
        }
    }

    /// Reads the received data chunk
    pub fn read_chunk(&mut self, bytes: Vec<u8>, length: usize) {
        match self.next_chunk_type {
            ChunkType::CommandId => {
                let id = Self::read_le_i32(&mut bytes[0..(length)].as_ref());
                self.id_received(id);
            },
            ChunkType::DataLength => {
                let len = Self::read_le_i32(&mut bytes[0..(length)].as_ref());
                self.data_length_received(len);
            },
            ChunkType::StringLength => {
                let str_len = Self::read_le_i32(&mut bytes[0..(length)].as_ref());
                self.string_length_received(str_len);
            },
            ChunkType::Data => {
                self.data_chunk_received(&bytes, &length);
            },
        }
    }

    pub async fn send_get_state(&mut self, state_id: i32) {
        // add this id to the expected responses array
        self.expected_responses.push(state_id);

        let mut ids_lock = self.id_queue.lock().await;
        let mut bool_lock = self.bool_queue.lock().await;

        ids_lock.add(state_id).unwrap();
        bool_lock.add(false).unwrap();
    }

    pub async fn send_set_state(&self, state_id: i32, value: TypedValue) {
        let mut ids_lock = self.id_queue.lock().await;
        let mut bool_lock = self.bool_queue.lock().await;
        let mut values_lock = self.value_queue.lock().await;

        ids_lock.add(state_id).unwrap();
        bool_lock.add(true).unwrap();
        values_lock.add(value).unwrap();
    }

    pub async fn send_command(&self, command_id: i32) {
        let mut ids_lock = self.id_queue.lock().await;
        let mut bool_lock = self.bool_queue.lock().await;

        ids_lock.add(command_id).unwrap();
        bool_lock.add(false).unwrap();
    }

    pub fn set_received_data_callback<F: Fn(ReceivedDataArgs) + Send + 'static>(&mut self, func: Option<Box<dyn Fn(ReceivedDataArgs) + Send + 'static>>)
    {
        self.data_received_callback = func;
    }

    pub fn set_received_manifest_callback<F: Fn(ReceivedManifestArgs) + Send + 'static>(&mut self, func: Option<Box<dyn Fn(ReceivedManifestArgs) + Send + 'static>>)
    {
        self.manifest_received_callback = func;
    }

    fn id_received(&mut self, id: i32) {
        // if present, remove this id from the expected responses array.
        // if this id is not expected, print a warning.
        if !self.expected_responses.contains(&id) {
            println!("Received an unexpected response from API, reading it anyway.");
        } else {
            for (index, expected_response_id) in self.expected_responses.iter().enumerate() {
                if id == *expected_response_id {
                    self.expected_responses.swap_remove(index);
                    break;
                }
            }
        }

        self.current_id = id;
        self.next_chunk_type = ChunkType::DataLength;
    }

    fn data_length_received(&mut self, data_length: i32) {
        self.current_data_length = data_length;

        let manifest = &self.manifest;

        // determine the next expected chunk based on the command id
        if self.current_id == -1 {
            self.next_chunk_type = ChunkType::StringLength;
        } else {
            let curr_datatype = manifest.as_ref().unwrap().get_data_type_for_id(&self.current_id);
            if curr_datatype.is_ok() {
                if curr_datatype.as_ref().unwrap() == &Type::String {
                    self.next_chunk_type = ChunkType::StringLength;
                } else {
                    self.next_chunk_type = ChunkType::Data;
                }
            }
        }
    }

    fn string_length_received(&mut self, string_length: i32) {
        self.current_string_len = string_length;
        self.next_chunk_type = ChunkType::Data;
    }

    fn data_chunk_received(&mut self, bytes: &Vec<u8>, bytes_length: &usize) {
        if self.manifest.is_none() || self.current_id == -1 {
            self.manifest_chunk_received(bytes, bytes_length);
        } else {
            let manifest = self.manifest.as_ref().unwrap();

            match manifest.get_data_type_for_id(&self.current_id).unwrap() {
                Type::String => self.string_chunk_received(bytes, bytes_length),
                Type::Long => self.long_received(bytes, bytes_length),
                Type::Boolean => self.boolean_received(bytes, bytes_length),
                Type::Integer32 => self.i32_received(bytes, bytes_length),
                Type::Float => self.float_received(bytes, bytes_length),
                Type::Double => self.double_received(bytes, bytes_length),
            }
        }
    }

    fn manifest_chunk_received(&mut self, bytes: &Vec<u8>, bytes_length: &usize) {
        let str_chunk = String::from_utf8_lossy(&bytes[0..*(bytes_length)]);
        self.current_data_string = self.current_data_string.to_owned() + str_chunk.as_ref();

        if self.current_data_string.as_bytes().len() >= self.current_string_len as usize {
            //println!("done reading manifest: {} bytes total", self.current_data_string.as_bytes().len());

            // parse the manifest
            let manifest = Manifest::from_str(&self.current_data_string);
            self.manifest = Some(manifest.clone());

            if let Some(callback) = &self.manifest_received_callback {
                callback(ReceivedManifestArgs::new(manifest.clone()))
            }
        }
    }

    fn string_chunk_received(&mut self, bytes: &Vec<u8>, bytes_length: &usize) {
        let str_chunk = String::from_utf8_lossy(&bytes[0..*(bytes_length)]);
        self.current_data_string = self.current_data_string.to_owned() + str_chunk.as_ref();

        if self.current_data_string.as_bytes().len() >= self.current_string_len as usize {
            //println!("done reading string: {} bytes total", self.current_data_string.as_bytes().len());

            if let Some(callback) = &self.data_received_callback {
                callback(ReceivedDataArgs::new(self.current_id, TypedValue::String(self.current_data_string.clone())))
            }

            //println!("full string: \n{}", self.current_data_string);
        }
    }

    fn double_received(&mut self, bytes: &Vec<u8>, bytes_length: &usize) {
        let data = Self::read_le_f64(&mut bytes[0..*(bytes_length)].as_ref());

        if let Some(callback) = &self.data_received_callback {
            callback(ReceivedDataArgs::new(self.current_id, TypedValue::Double(data)))
        }
    }

    fn float_received(&mut self, bytes: &Vec<u8>, bytes_length: &usize) {
        let data = Self::read_le_f32(&mut bytes[0..*(bytes_length)].as_ref());

        if let Some(callback) = &self.data_received_callback {
            callback(ReceivedDataArgs::new(self.current_id, TypedValue::Float(data)))
        }
    }

    fn i32_received(&mut self, bytes: &Vec<u8>, bytes_length: &usize) {
        let data = Self::read_le_i32(&mut bytes[0..*(bytes_length)].as_ref());

        if let Some(callback) = &self.data_received_callback {
            callback(ReceivedDataArgs::new(self.current_id, TypedValue::Integer32(data)))
        }
    }

    fn boolean_received(&mut self, bytes: &Vec<u8>, bytes_length: &usize) {
        let data = Self::read_bool(&mut bytes[0..*(bytes_length)].as_ref());

        if let Some(callback) = &self.data_received_callback {
            callback(ReceivedDataArgs::new(self.current_id, TypedValue::Boolean(data)))
        }
    }

    fn long_received(&mut self, bytes: &Vec<u8>, bytes_length: &usize) {
        let data = Self::read_le_i64(&mut bytes[0..*(bytes_length)].as_ref());

        if let Some(callback) = &self.data_received_callback {
            callback(ReceivedDataArgs::new(self.current_id, TypedValue::Long(data)))
        }
    }

    pub async fn send(&mut self, tcp_stream: &TcpStream) -> Result<(), Error> {
        // ensure the queues can be locked, otherwise skip
        let ids_lock = self.id_queue.try_lock();
        let bool_lock = self.bool_queue.try_lock();
        let values_lock = self.value_queue.try_lock();
        if ids_lock.is_err() || bool_lock.is_err() || values_lock.is_err() { return Ok(()) }

        // if there is nothing to write, skip
        if ids_lock.as_ref().unwrap().size() == 0 && bool_lock.as_ref().unwrap().size() == 0 && values_lock.as_ref().unwrap().size() == 0 {
            return Ok(())
        }

        // write id
        let mut id_queue = ids_lock.unwrap();
        let next_id_entry = id_queue.remove();
        if next_id_entry.is_ok() {
            match tcp_stream.try_write(&next_id_entry.unwrap().to_le_bytes()) {
                Ok(_n) => {
                    //println!("write {} id bytes", n);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    //return Ok(())
                }
                Err(_e) => {
                    //return Err(e.into())
                }
            }
        }

        // write bool
        let mut bool_queue = bool_lock.unwrap();
        let next_bool_entry = bool_queue.remove();
        if next_bool_entry.is_ok() {
            match tcp_stream.try_write(&(next_bool_entry.unwrap() as i32).to_le_bytes()) {
                Ok(_n) => {
                    //println!("write {} bool bytes", n);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    //return Ok(())
                }
                Err(_e) => {
                    //return Err(e.into())
                }
            }
        }

        // write value
        let mut values_queue = values_lock.unwrap();
        let next_values_entry = values_queue.remove();
        if next_values_entry.is_ok() {
            match tcp_stream.try_write(&(next_values_entry.unwrap()).to_bytes_vec()) {
                Ok(_n) => {
                    //println!("write {} value bytes", n);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    //return Ok(())
                }
                Err(_e) => {
                    //return Err(e.into())
                }
            }
        }

        Ok(())
    }

    pub fn get_manifest(&self) -> Result<&Manifest, ManifestError> {
        if self.manifest.is_some() {
            Ok(self.manifest.as_ref().unwrap())
        } else {
            Err(ManifestError::NoManifest())
        }
    }

    fn read_le_i32(input: &mut &[u8]) -> i32 {
        let (bytes, rest) = input.split_at(std::mem::size_of::<i32>());
        *input = rest;
        i32::from_le_bytes(bytes.try_into().unwrap())
    }

    fn read_le_f32(input: &mut &[u8]) -> f32 {
        let (bytes, rest) = input.split_at(std::mem::size_of::<f32>());
        *input = rest;
        f32::from_le_bytes(bytes.try_into().unwrap())
    }

    fn read_le_f64(input: &mut &[u8]) -> f64 {
        let (bytes, rest) = input.split_at(std::mem::size_of::<f64>());
        *input = rest;
        f64::from_le_bytes(bytes.try_into().unwrap())
    }

    fn read_le_i64(input: &mut &[u8]) -> i64 {
        let (bytes, rest) = input.split_at(std::mem::size_of::<i64>());
        *input = rest;
        i64::from_le_bytes(bytes.try_into().unwrap())
    }

    fn read_bool(input: &mut &[u8]) -> bool {
        input.ends_with(&[1u8])
    }
}
