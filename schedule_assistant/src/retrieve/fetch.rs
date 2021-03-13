//@todo: Create a trait to define the endpoint?

use reqwest::blocking::{Client, Response};
//use reqwest::header::*; //< for header.CONTENT_DISPOSITION
use serde_json::Value;

use super::authentication::Authentication;
use super::endpoint::Endpoint;

fn fetch<T: Endpoint, U: Authentication>(
    endpoint: &T,
    authentication: &U,
) -> reqwest::Result<Response> {
    let client = Client::new();
    let url = endpoint.url();
    println!("url: {}", url);
    let mut request_builder = client.get(&url);
    request_builder = authentication.apply(request_builder);
    request_builder.send()
}

pub fn get<T: Endpoint, U: Authentication>(
    endpoint: &T,
    authentication: &U,
) -> reqwest::Result<Value> {
    let response = fetch(endpoint, authentication)?;
    response.json()
}

pub fn get_list<T: Endpoint, U: Authentication>(
    endpoint: &T,
    authentication: &U,
) -> reqwest::Result<Vec<Value>> {
    let mut output = Vec::new();
    let value = get(endpoint, authentication)?;
    if value.is_array() {
        let list = value.as_array().unwrap();
        output.extend(list.clone().into_iter());
    } else {
        output.push(value);
    }
    Ok(output)
}

pub fn post<T: Endpoint, U: Authentication>(
    endpoint: &T,
    authentication: &U,
) -> reqwest::Result<Value> {
    let client = Client::new();
    let url = endpoint.url();
    println!("url: {}", url);
    let mut request_builder = client.post(&url);
    request_builder = authentication.apply(request_builder);
    let response = request_builder.send()?;
    response.json()
}

//document_id: u32
//let url = format!("{}/opportunity_documents/{}.pdf", BASE_URL, document_id);
// pub fn retrieve_binary<T: Endpoint, U: Authentication>(source: &Source, endpoint: &str) -> reqwest::Result<(Vec<u8>, String)> {
//     let mut response = source.get(endpoint)?;

//     // Retrieve filename;
//     let mut filename = String::from("picking_list.pdf");
//     if let Some(content_disposition) = response.headers().get(CONTENT_DISPOSITION) {
//         let mut str = content_disposition.to_str().unwrap();
//         if let Some(c) = str.find('"') {
//             str = &str[(c + 1)..];
//             if let Some(c) = str.find('"') {
//                 str = &str[..c];
//             }
//             filename = str.to_string();
//         }
//     }

//     let mut buffer = Vec::new();
//     response.copy_to(&mut buffer)?;
//     Ok((buffer, filename))
// }
