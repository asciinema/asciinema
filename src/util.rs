use std::path::{Path, PathBuf};
use std::{io, thread};

use anyhow::{anyhow, bail, Result};
use reqwest::Url;
use tempfile::NamedTempFile;

use crate::html;

pub fn get_local_path(filename: &str) -> Result<Box<dyn AsRef<Path>>> {
    if filename.starts_with("https://") || filename.starts_with("http://") {
        match download_asciicast(filename) {
            Ok(path) => Ok(Box::new(path)),
            Err(e) => bail!(anyhow!("download failed: {e}")),
        }
    } else {
        Ok(Box::new(PathBuf::from(filename)))
    }
}

fn download_asciicast(url: &str) -> Result<NamedTempFile> {
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

pub struct JoinHandle(Option<thread::JoinHandle<()>>);

impl JoinHandle {
    pub fn new(handle: thread::JoinHandle<()>) -> Self {
        Self(Some(handle))
    }
}

impl Drop for JoinHandle {
    fn drop(&mut self) {
        self.0
            .take()
            .unwrap()
            .join()
            .expect("worker thread should finish cleanly");
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

#[cfg(test)]
mod tests {
    use super::Utf8Decoder;

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
}
