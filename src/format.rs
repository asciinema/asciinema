pub mod asciicast;
pub mod raw;
use std::io;

pub trait Writer {
    fn header(&mut self, header: &Header) -> io::Result<()>;
    fn output(&mut self, time: f64, data: &[u8]) -> io::Result<()>;
    fn input(&mut self, time: f64, data: &[u8]) -> io::Result<()>;
}

pub struct Header {
    pub cols: u16,
    pub rows: u16,
    pub timestamp: u64,
    pub idle_time_limit: Option<f32>,
}
