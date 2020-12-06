//@todo: Combine the get, post, delete functions into a single function which takes an
//       enum and calls the appropriate function on the client.
//@todo: Should the program really panic! if we dont have a token?
//@todo: Handle the "Sale" items during Finalise.

use reqwest::blocking::Client;
use std::env;
use url::Url;

static BASE_URL: &'static str = "https://api.current-rms.com/api/v1";

pub fn get_opportunity(opportunity_id: u32) -> reqwest::Result<serde_json::Value> {
    let url = format!("{}/opportunities/{}", BASE_URL, opportunity_id);
    get(&url)
}

pub fn get_opportunities(page: &u32, per_page: &u32) -> reqwest::Result<serde_json::Value> {
    let url = format!(
        "{}/opportunities?page={}&per_page={}",
        BASE_URL, page, per_page
    );
    get(&url)
}

pub fn get(url: &str) -> Result<serde_json::Value, reqwest::Error> {
    let domain = env::var("CURRENT_DOMAIN_NAME").expect("CURRENT_DOMAIN_NAME not found");
    let password = env::var("CURRENT_ACCESS_TOKEN").expect("CURRENT_ACCESS_TOKEN not found");
    let encoded_url = Url::parse(&url).unwrap();
    let client = Client::new();
    let response = client
        .get(encoded_url)
        .header("X-SUBDOMAIN", domain)
        .header("X-AUTH-TOKEN", password)
        .send()?;

    response.json()
}
