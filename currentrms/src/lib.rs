//@todo: Combine the get, post, delete functions into a single function which takes an
//       enum and calls the appropriate function on the client.
//@todo: Should the program really panic! if we dont have a token?
//@todo: Handle the "Sale" items during Finalise.

use reqwest::blocking::Client;
use reqwest::header::*;
use std::env;
use url::Url;

static BASE_URL: &str = "https://api.current-rms.com/api/v1";

pub enum DataType {
    Opportunity,
    OpportunityDocuments,
    Members,
}

pub fn retrieve(data_type: DataType, id: u32, secondary: Option<u32>) -> reqwest::Result<serde_json::Value> {
    let url = match data_type {
        DataType::Opportunity => format!("{}/opportunities/{}", BASE_URL, id),
        DataType::OpportunityDocuments => format!("{}/opportunities/{}/opportunity_documents/{}", BASE_URL, id, secondary.unwrap()), 
        DataType::Members => format!("{}/members/{}", BASE_URL, id),
    };
    get(&url)
}

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

pub fn get_member(id: u32) -> reqwest::Result<serde_json::Value> {
    let url = format!("{}/members/{}", BASE_URL, id);
    get(&url)
}

pub fn get_opportunity_documents(
    page: u32,
    per_page: u32,
    opportunity_id: u32,
) -> reqwest::Result<serde_json::Value> {
    let url = format!(
        "{}/opportunity_documents?page={}&per_page={}&opportunity_id={}",
        BASE_URL, page, per_page, opportunity_id
    );
    get(&url)
}

pub fn get_opportunity_document_pdf(document_id: u32) -> reqwest::Result<(Vec<u8>, String)> {
    let domain = env::var("CURRENT_DOMAIN_NAME").expect("CURRENT_DOMAIN_NAME not found");
    let password = env::var("CURRENT_ACCESS_TOKEN").expect("CURRENT_ACCESS_TOKEN not found");
    let url = format!("{}/opportunity_documents/{}.pdf", BASE_URL, document_id);
    let encoded_url = Url::parse(&url).unwrap();
    let client = Client::new();
    let mut response = client
        .get(encoded_url)
        .header("X-SUBDOMAIN", domain)
        .header("X-AUTH-TOKEN", password)
        .send()?;

    // Retrieve filename;
    let mut filename = String::from("picking_list.pdf");
    if let Some(content_disposition) = response.headers().get(CONTENT_DISPOSITION) {
        let mut str = content_disposition.to_str().unwrap();
        if let Some(c) = str.find('"') {
            str = &str[(c+1)..];
            if let Some(c) = str.find('"') {
                str = &str[..c];
            }
            filename = str.to_string();
        }
    }

    let mut buffer = Vec::new();
    response.copy_to(&mut buffer)?;
    Ok((buffer, filename))
}

pub fn print_opportunity_document_pdf(opportunity_id: u32, document_id: u32) -> reqwest::Result<(Vec<u8>, String)> {
    let domain = env::var("CURRENT_DOMAIN_NAME").expect("CURRENT_DOMAIN_NAME not found");
    let password = env::var("CURRENT_ACCESS_TOKEN").expect("CURRENT_ACCESS_TOKEN not found");
    let url = format!("{}/opportunities/{}/print_document.pdf?document_id={}.pdf", BASE_URL, opportunity_id, document_id);
    let encoded_url = Url::parse(&url).unwrap();
    println!("{}", url);
    let client = Client::new();
    let mut response = client
        .get(encoded_url)
        .header("X-SUBDOMAIN", domain)
        .header("X-AUTH-TOKEN", password)
        .send()?;

    // Retrieve filename;
    let mut filename = String::from("picking_list.pdf");
    if let Some(content_disposition) = response.headers().get(CONTENT_DISPOSITION) {
        let mut str = content_disposition.to_str().unwrap();
        if let Some(c) = str.find('"') {
            str = &str[(c+1)..];
            if let Some(c) = str.find('"') {
                str = &str[..c];
            }
            filename = str.to_string();
        }
    }

    let mut buffer = Vec::new();
    response.copy_to(&mut buffer)?;
    Ok((buffer, filename))
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
