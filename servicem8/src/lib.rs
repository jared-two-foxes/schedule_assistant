use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use url::Url;
use uuid::Uuid;

static BASE_URL: &str = "https://api.servicem8.com/api_1.0";

mod date_format {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Job {
    pub uuid: Uuid,
    pub active: u32,
    #[serde(with = "date_format")]
    pub date: DateTime<Utc>,
    pub company_uuid: Uuid,
    pub job_address: String,
    pub status: String,
}

pub fn get_clients() -> Result<Value, reqwest::Error> {
    let url = format!("{}/company.json", BASE_URL);
    get(&url)
}

pub fn get_client(uuid: &str) -> Result<Value, reqwest::Error> {
    let url = format!("{}/company/{}.json", BASE_URL, uuid);
    get(&url)
}

pub fn get_jobs(filter: Option<&str>) -> Result<Value, reqwest::Error> {
    let url = match filter {
        Some(f) => format!("{}/job.json?$filter={}", BASE_URL, f),
        _ => format!("{}/job.json", BASE_URL),
    };
    get(&url)
}

pub fn get_job(uuid: &str) -> Result<Value, reqwest::Error> {
    let url = format!("{}/job/{}.json", BASE_URL, uuid);
    get(&url)
}

pub fn update_jobs(
    uuid: &str,
    value: Value,
) -> Result<Value, reqwest::Error> {
    let url = format!("{}/job/{}.json", BASE_URL, uuid);
    post(&url, value)
}

pub fn get_job_activities(filter: Option<&str>) -> Result<Value, reqwest::Error> {
    let url = match filter {
        Some(f) => format!("{}/jobactivity.json?$filter={}", BASE_URL, f),
        _ => format!("{}/jobactivity.json", BASE_URL),
    };
    get(&url)
}


pub fn get(url: &str) -> Result<Value, reqwest::Error> {
    let username = env::var("SERVICEM8_USERNAME").expect("SERVICEM8_USERNAME not found");
    let password = env::var("SERVICEM8_PASSWORD").expect("SERVICEM8_PASSWORD not found");
    let encoded_url = Url::parse(&url).unwrap();
    let client = Client::new();
    let response = client
        .get(encoded_url)
        .basic_auth(username, Some(password))
        .send()?;

    response.json()
}

pub fn post(url: &str, post: Value) -> Result<Value, reqwest::Error> {
    let username = env::var("SERVICEM8_USERNAME").expect("SERVICEM8_USERNAME not found");
    let password = env::var("SERVICEM8_PASSWORD").expect("SERVICEM8_PASSWORD not found");
    let encoded_url = Url::parse(&url).unwrap();
    let client = Client::new();
    let response = client
        .post(encoded_url)
        .basic_auth(username, Some(password))
        .json(&post)
        .send()?;

    response.json()
}
