use std::io;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};
use reqwest::Url;
use tempfile::NamedTempFile;

use crate::html;

pub fn get_local_path(filename: &str) -> anyhow::Result<Box<dyn AsRef<Path>>> {
    if filename.starts_with("https://") || filename.starts_with("http://") {
        match download_asciicast(filename) {
            Ok(path) => Ok(Box::new(path)),
            Err(e) => bail!(anyhow!("download failed: {e}")),
        }
    } else {
        Ok(Box::new(PathBuf::from(filename)))
    }
}

fn download_asciicast(url: &str) -> anyhow::Result<NamedTempFile> {
    use reqwest::blocking::get;

    let mut response = get(Url::parse(url)?)?;
    response.error_for_status_ref()?;
    let mut file = NamedTempFile::new()?;

    let content_type = response
        .headers()
        .get("content-type")
        .ok_or(anyhow!("no content-type header in the response"))?
        .to_str()?;

    if content_type.starts_with("text/html") {
        if let Some(url) = html::extract_asciicast_link(&response.text()?) {
            let mut response = get(Url::parse(&url)?)?;
            response.error_for_status_ref()?;
            io::copy(&mut response, &mut file)?;

            Ok(file)
        } else {
            bail!(
                r#"<link rel="alternate" type="application/x-asciicast" href="..."> not found in the HTML page"#
            );
        }
    } else {
        io::copy(&mut response, &mut file)?;

        Ok(file)
    }
}

pub struct Utf8Decoder(Vec<u8>);

impl Utf8Decoder {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn feed(&mut self, input: &[u8]) -> String {
        let mut output = String::new();
        self.0.extend_from_slice(input);

        while !self.0.is_empty() {
            match std::str::from_utf8(&self.0) {
                Ok(valid_str) => {
                    output.push_str(valid_str);
                    self.0.clear();
                    break;
                }

                Err(e) => {
                    let n = e.valid_up_to();
                    let valid_bytes: Vec<u8> = self.0.drain(..n).collect();
                    let valid_str = unsafe { std::str::from_utf8_unchecked(&valid_bytes) };
                    output.push_str(valid_str);

                    match e.error_len() {
                        Some(len) => {
                            self.0.drain(..len);
                            output.push('�');
                        }

                        None => {
                            break;
                        }
                    }
                }
            }
        }

        output
    }
}

/// Quantizer using error diffusion based on Bresenham algorithm.
/// It ensures the accumulated error at any point is less than Q/2.
pub struct Quantizer {
    q: i128,
    error: i128,
}

impl Quantizer {
    pub fn new(q: u128) -> Self {
        Quantizer {
            q: q as i128,
            error: 0,
        }
    }

    pub fn next(&mut self, value: u128) -> u128 {
        let error_corrected_value = value as i128 + self.error;
        let steps = (error_corrected_value + self.q / 2) / self.q;
        let quantized_value = steps * self.q;

        self.error = error_corrected_value - quantized_value;
        debug_assert!((self.error).abs() <= self.q / 2);

        quantized_value as u128
    }
}

#[cfg(test)]
mod tests {
    use super::{Quantizer, Utf8Decoder};

    #[test]
    fn utf8_decoder() {
        let mut decoder = Utf8Decoder::new();

        assert_eq!(decoder.feed(b"czarna "), "czarna ");
        assert_eq!(decoder.feed(&[0xc5, 0xbc, 0xc3]), "ż");
        assert_eq!(decoder.feed(&[0xb3, 0xc5, 0x82]), "ół");
        assert_eq!(decoder.feed(&[0xc4]), "");
        assert_eq!(decoder.feed(&[0x87, 0x21]), "ć!");
        assert_eq!(decoder.feed(&[0x80]), "�");
        assert_eq!(decoder.feed(&[]), "");
        assert_eq!(decoder.feed(&[0x80, 0x81]), "��");
        assert_eq!(decoder.feed(&[]), "");
        assert_eq!(decoder.feed(&[0x23]), "#");
        assert_eq!(
            decoder.feed(&[0x83, 0x23, 0xf0, 0x90, 0x80, 0xc0, 0x21]),
            "�#��!"
        );
    }

    #[test]
    fn quantizer() {
        let mut quantizer = Quantizer::new(1_000);

        let input = [
            026692, 540290, 064736, 105951, 171006, 191943, 107942, 128108, 148904, 108973, 211002,
            044701, 489307, 405987, 105028, 194590, 061043, 532296, 319015, 152786, 032578, 005445,
            040542, 000756,
        ];

        let expected = [
            27000, 540000, 65000, 106000, 171000, 192000, 108000, 128000, 149000, 109000, 211000,
            44000, 490000, 406000, 105000, 194000, 61000, 532000, 320000, 152000, 33000, 5000,
            41000, 1000,
        ];

        let mut quantized = Vec::new();
        let mut input_sum = 0;
        let mut quantized_sum = 0;

        for input_value in input {
            let quantized_value = quantizer.next(input_value);
            quantized.push(quantized_value);
            input_sum += input_value;
            quantized_sum += quantized_value;
            let error = (input_sum as i128 - quantized_sum as i128).abs();

            assert!(error <= 500, "error: {error}");
        }

        assert_eq!(quantized, expected);
    }
}
