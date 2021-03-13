use crate::authentication;
use crate::endpoints::servicem8;
use crate::json;
use crate::retrieve::fetch;
use serde_json::Value;

// [Retrieval]
pub fn clients() -> reqwest::Result<Vec<Value>> {
    let endpoint = servicem8::clients();
    let authentication = authentication::servicem8();
    let list = fetch::get_list(&endpoint, &authentication)?;
    println!("Found {} clients", list.len());
    Ok(list)
}

pub fn job_activities() -> reqwest::Result<Vec<Value>> {
    let endpoint = servicem8::activities();
    let authentication = authentication::servicem8();
    let list = fetch::get_list(&endpoint, &authentication)?;
    println!("Found {} clients", list.len());
    Ok(list)
}

pub fn jobs() -> reqwest::Result<Vec<Value>> {
    let endpoint = servicem8::jobs();
    let authentication = authentication::servicem8();
    let list = fetch::get_list(&endpoint, &authentication)?;
    println!("Found {} clients", list.len());
    Ok(list)
}

pub fn job_contacts() -> reqwest::Result<Vec<Value>> {
    let endpoint = servicem8::contacts();
    let authentication = authentication::servicem8();
    let list = fetch::get_list(&endpoint, &authentication)?;
    println!("Found {} clients", list.len());
    Ok(list)
}

pub fn activity_is_active(activity: &Value) -> bool {
    let value = json::attribute_from_value(activity, "active")
        .expect("Activity doesn't have an 'active' value");
    value == "1"
}
