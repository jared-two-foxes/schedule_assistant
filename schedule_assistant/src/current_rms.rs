use super::authentication;
use super::endpoints::current_rms;
use super::retrieve::fetch;
use serde_json::Value;

// [Retrieval]
// Retrieves opportunities which is active between the start and end dates.
pub fn opportunities() -> reqwest::Result<Vec<Value>> {
    let mut endpoint = current_rms::opportunities();
    let authentication = authentication::current_rms();
    let mut list = Vec::new();

    loop {
        let response = fetch::get(&endpoint, &authentication)?;
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

pub fn mark_as_lost(opportunity_id: u64) -> reqwest::Result<Value> {
    let endpoint = current_rms::mark_as_dead(opportunity_id);
    let authentication = authentication::current_rms();
    fetch::get(&endpoint, &authentication)
}
