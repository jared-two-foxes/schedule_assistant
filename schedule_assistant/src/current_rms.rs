use super::authentication::AuthenticationCache;
use super::endpoints::current_rms;
use super::retrieve::fetch;

use serde_json::Value;
use std::env;


// [Retrieval]
// Retrieves opportunities which is active between the start and end dates.
pub fn opportunities(auth_cache: &AuthenticationCache) -> reqwest::Result<Vec<Value>> {
    let mut endpoint = current_rms::opportunities();
    let authentication = auth_cache.currentrms();
    let mut list = Vec::new();

    loop {
        let response = fetch::get(&endpoint, authentication)?;
        // If there isnt an object then somethings gone wrong?
        let object = &response["opportunities"];
        if !object.is_array() {
            //@todo: Should this be an error?
            break;
        }

        // If there are no more objects then we're done.
        let objects = object.as_array().unwrap();
        if objects.is_empty() {
            break;
        }

        // Cloning all the objects found and push all these structures into a vector to
        // be returned.
        list.extend(objects.clone().into_iter());
        endpoint.advance();
    }

    println!("Found {} opportunities", list.len());
    Ok(list)
}

pub fn mark_as_lost(auth_cache: &AuthenticationCache, opportunity_id: u64) -> reqwest::Result<Value> {
    let endpoint = current_rms::mark_as_dead(opportunity_id);
    let authentication = auth_cache.currentrms();
    fetch::get(&endpoint, authentication)
}

pub fn opportunity_is_confirmed(op: &Value) -> bool {
    op["state"].as_u64().unwrap_or(0) == 3
}

pub fn print_document_pdf(
    auth_cache: &AuthenticationCache,
    opportunity_id: u64,
    document_id: u64,
) -> reqwest::Result<(Vec<u8>, String)> {
    use reqwest::header::*;
    let authentication = auth_cache.currentrms();
    let subdomain = env::var("CURRENT_DOMAIN_NAME").expect("CURRENT_DOMAIN_NAME not found");
    let endpoint = current_rms::opportunity_print_document_pdf(&subdomain, opportunity_id, document_id);

    // Retrieve filename;
    let mut filename = String::from("picking_list.pdf");
    let mut response = fetch::fetch(&endpoint, authentication)?;
    if let Some(content_disposition) = response.headers().get(CONTENT_DISPOSITION) {
        let mut str = content_disposition.to_str().unwrap();
        if let Some(c) = str.find('"') {
            str = &str[(c + 1)..];
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
