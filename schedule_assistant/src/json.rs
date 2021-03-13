
use chrono::prelude::*;
use serde_json::Value;

pub fn attribute_from_value(value: &Value, attribute: &str) -> Option<String> {
    let element = value[attribute].as_str()?;
    if element == "" {
        return None;
    }
    Some(String::from(element))
}

pub fn date_from_value(value: &Value, attribute: &str, fmt: &str) -> Option<DateTime<Utc>> {
    let element = attribute_from_value(value, attribute)?;
    Utc.datetime_from_str(&element, fmt).ok()
}