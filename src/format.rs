pub mod asciicast;
pub mod raw;
use std::{collections::HashMap, io};

pub trait Writer {
    fn header(&mut self, header: &Header) -> io::Result<()>;
    fn output(&mut self, time: u64, data: &[u8]) -> io::Result<()>;
    fn input(&mut self, time: u64, data: &[u8]) -> io::Result<()>;
    fn resize(&mut self, time: u64, size: (u16, u16)) -> io::Result<()>;
}

pub struct Header {
    pub cols: u16,
    pub rows: u16,
    pub timestamp: u64,
    pub idle_time_limit: Option<f64>,
    pub command: Option<String>,
    pub title: Option<String>,
    pub env: HashMap<String, String>,
}
