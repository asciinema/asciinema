pub mod asciicast;
pub mod raw;
use std::io;

pub trait Writer {
    fn header(&mut self, size: (u16, u16), idle_time_limit: Option<f32>) -> io::Result<()>;
    fn output(&mut self, time: f64, data: &[u8]) -> io::Result<()>;
    fn input(&mut self, time: f64, data: &[u8]) -> io::Result<()>;
}
