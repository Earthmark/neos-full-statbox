use serde::{Deserialize, Deserializer};

use super::RcStr;

pub fn null_to_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Deserialize::deserialize(d).map(|o: Option<T>| o.unwrap_or_default())
}

pub fn err_to_none<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Deserialize::deserialize(d).ok())
}

pub fn option_split_backslashes<'de, D>(d: D) -> Result<Vec<RcStr>, D::Error>
where
    D: Deserializer<'de>,
{
    let str: Option<String> = Deserialize::deserialize(d)?;
    Ok(str
        .map(|s| s.split("\\").map(|s| s.to_owned().into()).collect())
        .unwrap_or_default())
}
