// This module should handle the communication out to the various web servers
// etc that are being used.

use reqwest::blocking::Client;
use url::Url;

pub fn get(url: &str) -> reqwest::Result<serde_json::Value> {
    let client = Client::new();
    let encoded_url = Url::parse(&url).unwrap();
    let response = client
        .get(encoded_url)
        // .header("X-SUBDOMAIN", domain)
        // .header("X-AUTH-TOKEN", password)
        .send()?;

    //println!( "{:?}", &response );
    response.json()
}
