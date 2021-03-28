use crate::authentication::AuthenticationCache;
use crate::endpoints::servicem8;
use crate::retrieve::fetch;
use serde_json::Value;

// [Retrieval]
pub fn clients(auth_cache: &AuthenticationCache) -> reqwest::Result<Vec<Value>> {
    let endpoint = servicem8::clients();
    let authentication = auth_cache.servicem8();
    let list = fetch::get_list(&endpoint, authentication)?;
    println!("Found {} clients", list.len());
    Ok(list)
}

pub fn job_activities(auth_cache: &AuthenticationCache) -> reqwest::Result<Vec<Value>> {
    let endpoint = servicem8::activities();
    let authentication = auth_cache.servicem8();
    let list = fetch::get_list(&endpoint, authentication)?;
    println!("Found {} activities", list.len());
    Ok(list)
}

pub fn jobs(auth_cache: &AuthenticationCache) -> reqwest::Result<Vec<Value>> {
    let endpoint = servicem8::jobs();
    let authentication = auth_cache.servicem8();
    let list = fetch::get_list(&endpoint, authentication)?;
    println!("Found {} jobs", list.len());
    Ok(list)
}

pub fn job_contacts(auth_cache: &AuthenticationCache) -> reqwest::Result<Vec<Value>> {
    let endpoint = servicem8::contacts();
    let authentication = auth_cache.servicem8();
    let list = fetch::get_list(&endpoint, authentication)?;
    println!("Found {} contacts", list.len());
    Ok(list)
}

pub fn activity_is_active(activity: &Value) -> bool {
    let value = activity["active"].as_u64()
         .expect("Activity doesn't have an 'active' value");
    value == 1
}
