use anyhow::Result;
use serde::{Deserialize, Deserializer};

pub fn deserialize_time<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;

    let number = value
        .as_f64()
        .map(|v| v.to_string())
        .ok_or(Error::custom("expected number"))?;

    let parts: Vec<&str> = number.split('.').collect();

    match parts.as_slice() {
        [left, right] => {
            let secs: u64 = left.parse().map_err(Error::custom)?;
            let right = right.trim();

            let micros: u64 = format!("{:0<6}", &right[..(6.min(right.len()))])
                .parse()
                .map_err(Error::custom)?;

            Ok(secs * 1_000_000 + micros)
        }

        [number] => {
            let secs: u64 = number.parse().map_err(Error::custom)?;

            Ok(secs * 1_000_000)
        }

        _ => Err(Error::custom(format!("invalid time format: {value}"))),
    }
}
