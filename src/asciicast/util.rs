use anyhow::Result;
use serde::{Deserialize, Deserializer};

pub fn deserialize_time<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
    let string = value.as_f64().map(|v| v.to_string()).unwrap_or_default();
    let parts: Vec<&str> = string.split('.').collect();

    match parts.as_slice() {
        [left, right] => {
            let secs: u64 = left.parse().map_err(Error::custom)?;
            let right = right.trim();

            let micros: u64 = format!("{:0<6}", &right[..(6.min(right.len()))])
                .parse()
                .map_err(Error::custom)?;

            Ok(secs * 1_000_000 + micros)
        }

        _ => Err(Error::custom("invalid time format")),
    }
}
