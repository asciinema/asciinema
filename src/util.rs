use anyhow::{anyhow, bail, Result};
use reqwest::Url;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

pub enum LocalPath {
    Normal(PathBuf),
    Temporary(NamedTempFile),
}

impl AsRef<Path> for LocalPath {
    fn as_ref(&self) -> &Path {
        match self {
            LocalPath::Normal(p) => p,
            LocalPath::Temporary(f) => f.path(),
        }
    }
}

pub fn get_local_path(filename: &str) -> Result<LocalPath> {
    if filename.starts_with("https://") || filename.starts_with("http://") {
        download_asciicast(filename)
            .map(LocalPath::Temporary)
            .map_err(|e| anyhow!("download failed: {e}"))
    } else {
        Ok(LocalPath::Normal(PathBuf::from(filename)))
    }
}

const LINK_REL_SELECTOR: &str = r#"link[rel="alternate"][type="application/x-asciicast"], link[rel="alternate"][type="application/asciicast+json"]"#;

fn download_asciicast(url: &str) -> Result<NamedTempFile> {
    use reqwest::blocking::get;
    use scraper::{Html, Selector};

    let mut response = get(Url::parse(url)?)?;
    response.error_for_status_ref()?;
    let mut file = NamedTempFile::new()?;

    let content_type = response
        .headers()
        .get("content-type")
        .ok_or(anyhow!("no content-type header in the response"))?
        .to_str()?;

    if content_type.starts_with("text/html") {
        let document = Html::parse_document(&response.text()?);
        let selector = Selector::parse(LINK_REL_SELECTOR).unwrap();
        let mut elements = document.select(&selector);

        if let Some(url) = elements.find_map(|e| e.value().attr("href")) {
            let mut response = get(Url::parse(url)?)?;
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
