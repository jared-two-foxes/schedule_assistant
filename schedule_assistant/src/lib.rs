mod authentication;
mod endpoints;
pub mod json;
mod macros;
mod retrieve;

pub mod current_rms;
pub mod servicem8;

use chrono::{Date, DateTime, Utc};
use serde_json::Value;

pub fn check_bookings(start: Date<Utc>, end: Date<Utc>) -> reqwest::Result<Vec<Value>> {
    // Get Current-RMS opportunities.
    let opportunities = current_rms::opportunities()?
        .into_iter()
        .filter(|opportunity| {
            current_rms::opportunity_is_confirmed(opportunity)
                && current_rms::opportunity_within_date_range(opportunity, &start, &end).unwrap()
        })
        .collect::<Vec<Value>>();

    // Get ServiceM8 data that we're going to need.
    let jobs = servicem8::jobs()?;
    let clients = servicem8::clients()?;
    let job_activities = servicem8::job_activities()?;
    let job_contacts = servicem8::job_contacts()?;

    // Check that all the opportunities have jobs registered in servicem8 with allocated
    // Activities for delivery & collection.
    let unscheduled = opportunities
        .into_iter()
        .filter(|opportunity| {
            !current_rms::opportunity_has_job(
                &opportunity,
                &jobs,
                &clients,
                &job_contacts,
                &job_activities,
            )
        })
        .collect::<Vec<Value>>();

    Ok(unscheduled)
}

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
