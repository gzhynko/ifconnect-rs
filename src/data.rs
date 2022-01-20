use std::convert::TryInto;
use std::ops::Add;
use tokio::io::Error;
use tokio::net::TcpStream;
use crate::manifest::Manifest;
use crate::typed_value::Type;

pub enum ChunkType {
    CommandId,
    DataLength,
    StringLength,
    Data
}

pub struct ConnectionData {
    pub manifest: Option<Manifest>,
    next_chunk_type: ChunkType,
    current_command_id: i32,
    current_data_length: u32,
    current_string_len: u32,
    current_data_string: String,
}

impl Default for ConnectionData {
    fn default() -> Self {
        Self {
            manifest: Option::None,
            next_chunk_type: ChunkType::CommandId,
            current_command_id: 0,
            current_data_length: 0,
            current_string_len: 0,
            current_data_string: String::new(),
        }
    }
}

impl ConnectionData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&mut self, tcp_stream: &TcpStream) -> Result<(), Error> {
        let mut buf = Vec::<u8>::new();
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
                println!("read {} bytes", len);
                self.read_chunk(buf, len);

                Ok(())
            },
            // may still fail with `WouldBlock` if the readiness event is a false positive.
            Err(e) if e.kind() == tokio::io::ErrorKind::WouldBlock => {
                Err(e.into())
            },
            Err(e) => {
                Err(e.into())
            },
        }
    }

    /// Reads the received data chunk
    pub fn read_chunk(&mut self, bytes: Vec<u8>, length: usize) {
        match self.next_chunk_type {
            ChunkType::CommandId => {
                let id = Self::read_be_i32(&mut bytes[0..(length)].as_ref());
                self.command_id_received(id);
            },
            ChunkType::DataLength => {
                let len = Self::read_be_u32(&mut bytes[0..(length)].as_ref());
                self.data_length_received(len);
            },
            ChunkType::StringLength => {
                let str_len = Self::read_be_u32(&mut bytes[0..(length)].as_ref());
                self.string_length_received(str_len);
            },
            ChunkType::Data => {
                self.data_chunk_received(&bytes, &length);
            },
        }
    }

    fn command_id_received(&mut self, id: i32) {
        println!("command id: {}", &id);

        self.current_command_id = id;
        self.next_chunk_type = ChunkType::DataLength;
    }

    fn data_length_received(&mut self, data_length: u32) {
        println!("data length: {}", &data_length);

        self.current_data_length = data_length;

        let manifest = &self.manifest;

        // determine the next expected chunk based on the command id
        if manifest.is_none() {
            if self.current_command_id == -1 {
                self.next_chunk_type = ChunkType::StringLength;
            }
        } else {
            let curr_datatype = manifest.as_ref().unwrap().get_data_type(&self.current_command_id);
            if curr_datatype.is_ok() {
                if curr_datatype.as_ref().unwrap() == &Type::String {
                    self.next_chunk_type = ChunkType::StringLength;
                } else {
                    self.next_chunk_type = ChunkType::Data;
                }
            }
        }
    }

    fn string_length_received(&mut self, string_length: u32) {
        println!("string datatype, expected string length: {}", &string_length);

        self.current_string_len = string_length;
        self.next_chunk_type = ChunkType::Data;
    }

    fn data_chunk_received(&mut self, bytes: &Vec<u8>, bytes_length: &usize) {
        let str_chunk = String::from_utf8_lossy(&bytes[0..*(bytes_length)]);
        self.current_data_string = self.current_data_string.to_owned() + str_chunk.as_ref();

        if self.current_data_string.as_bytes().len() >= (self.current_string_len / 256) as usize {
            println!("finished reading: {} bytes", self.current_data_string.as_bytes().len());
            println!("full string: \n{}", self.current_data_string);
        }
    }

    /// https://doc.rust-lang.org/std/primitive.i32.html#method.from_be_bytes
    fn read_be_i32(input: &mut &[u8]) -> i32 {
        let (int_bytes, rest) = input.split_at(std::mem::size_of::<i32>());
        *input = rest;
        i32::from_be_bytes(int_bytes.try_into().unwrap())
    }

    fn read_be_u32(input: &mut &[u8]) -> u32 {
        let (int_bytes, rest) = input.split_at(std::mem::size_of::<u32>());
        *input = rest;
        u32::from_be_bytes(int_bytes.try_into().unwrap())
    }
}