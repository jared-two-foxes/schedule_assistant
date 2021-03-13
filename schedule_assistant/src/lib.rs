mod authentication;
mod endpoints;
pub mod json;
mod macros;
mod retrieve;

pub mod current_rms;
pub mod servicem8;

use chrono::{DateTime, Utc};

pub fn remove_expired_quotes(date: DateTime<Utc>) {
    // get all the opportunities
    let opportunities = current_rms::opportunities().unwrap();

    // filter for the opportunities in the quotation state
    opportunities
        .into_iter()
        .filter(|opportunity| {
            json::attribute_from_value(&opportunity, "state_name").unwrap() == "Quotation"
        })
        .for_each(|opportunity| {
            // kill all the quotes earlier than date
            //  If date is before 'X'
            match json::date_from_value(&opportunity, "starts_at", "%Y-%m-%dT%H:%M:%S%.3f%Z") {
                Some(start_date) => {
                    if start_date < date {
                        // cancel the opportunity, mark as dead?
                        //println!("{}", serde_json::to_string_pretty(&opportunity).unwrap());
                        //let opportunity_id = json::attribute_from_value(&opportunity, "id").unwrap();
                        let opportunity_id = opportunity["id"].as_u64().unwrap();
                        println!(
                            "Attempting to cancel unconfirmed quote number {}.",
                            opportunity_id
                        );
                        match current_rms::mark_as_lost(opportunity_id) {
                            Err(e) => println!("Err: Failed to mark the event as lost. {}", e),
                            _ => println!("Marked event as lost."),
                        }
                    }
                }
                None => {}
            }
        });
}
