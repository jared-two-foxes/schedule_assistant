use super::authentication;
use super::endpoints::current_rms;
use super::retrieve::fetch;
use super::{guard, json};

use chrono::{Date, Utc};
use serde_json::Value;

fn date_bound_by(date: &Date<Utc>, start: &Date<Utc>, end: &Date<Utc>) -> bool {
    date > start && date < end
}

type Period<T> = (Date<T>, Date<T>);
fn period_bound_by(period: &Period<Utc>, start: &Date<Utc>, end: &Date<Utc>) -> bool {
    date_bound_by(&period.0, start, end) && date_bound_by(&period.1, start, end)
}

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



fn member_matches_client(member: &Value, client: &Value) -> bool {
    let member_name = guard!(json::attribute_from_value(&member, "name"));
    let mut attribute = guard!(json::attribute_from_value(&client, "name"));
    let mut first_name = String::from("");
    let mut last_name = String::from("");

    if let Some(is_individual) = client["is_individual"].as_i64() {
        if is_individual == 1 {
            let c = attribute.find(',');
            match c {
                // client name is structured "lastname, firstname"
                Some(c) => {
                    last_name = attribute.drain(..c).collect();
                    first_name = attribute[2..].to_string();
                }
                // client name either has no surname or is structured
                // "firstname lastname"
                None => {
                    let c = attribute.find(' ').unwrap_or_else(|| attribute.len());
                    first_name = attribute.drain(..c).collect();
                    if !attribute.is_empty() {
                        last_name = attribute;
                    }
                }
            }
        } else {
            first_name = attribute;
        }
    }

    let mut  name = format!("{} {}", first_name, last_name ).to_string();
    name = name.trim().to_string();
    member_name == name
}

fn member_matches_contact(member: &Value, contact: &Value) -> bool {
    let first_name = guard!(json::attribute_from_value(&contact, "first"));
    let last_name = guard!(json::attribute_from_value(&contact, "last"));
    let full_name = format!("{} {}", first_name, last_name).trim().to_string();
    let email = guard!(json::attribute_from_value(&contact, "email"));
    let member_name = guard!(json::attribute_from_value(&member, "name"));
    let member_email = guard!(json::attribute_from_value(&member, "email"));

    member_name == full_name || member_email == email
}

pub fn activity_within_date_range(
    activity: &Value,
    start: &Date<Utc>,
    end: &Date<Utc>,
) -> Option<bool> {
    let starts_at = json::date_from_value(activity, "start_date", "%Y-%m-%d %H:%M:%S")?;
    let ends_at = json::date_from_value(activity, "end_date", "%Y-%m-%d %H:%M:%S")?;
    Some(period_bound_by(
        &(starts_at.date(), ends_at.date()),
        start,
        end,
    ))
}

pub fn opportunity_within_date_range(op: &Value, start: &Date<Utc>, end: &Date<Utc>) -> Option<bool> {
    let starts_at = json::date_from_value(op, "starts_at", "%Y-%m-%dT%H:%M:%S%.3f%Z")?;
    let ends_at = json::date_from_value(op, "ends_at", "%Y-%m-%dT%H:%M:%S%.3f%Z")?;
    Some(period_bound_by(
        &(starts_at.date(), ends_at.date()),
        start,
        end,
    ))
}


pub fn opportunity_is_confirmed(op: &Value) -> bool {
    op["state"].as_u64().unwrap_or(0) == 3 
}


// Job contacts should only be those associated with this job.
fn opportunity_matches_job(
    opportunity: &Value,
    client: &Value,
    job_activities: &Vec<&Value>,
    job_contacts: &Vec<&Value>,
) -> reqwest::Result<bool> {
    let member = &opportunity["member"];
    if !member.is_object() {
        return Ok(false);
    }

    let starts_at =
        json::date_from_value(opportunity, "starts_at", "%Y-%m-%dT%H:%M:%S%.3f%Z").unwrap();
    let ends_at = json::date_from_value(opportunity, "ends_at", "%Y-%m-%dT%H:%M:%S%.3f%Z").unwrap();
    for activity in job_activities {
        if let Some(value) =
            activity_within_date_range(&activity, &starts_at.date(), &ends_at.date())
        {
            if value {
                return Ok(true);
            }
        }
    }

    if member_matches_client(member, &client) {
        return Ok(true);
    }

    for contact in job_contacts {
        if member_matches_contact(member, &contact) {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn opportunity_has_job(
    opportunity: &Value,
    jobs: &Vec<Value>,
    clients: &Vec<Value>,
    contacts: &Vec<Value>,
    activities: &Vec<Value>,
) -> bool {
    jobs.iter()
        .find(|&job| {
            let job_uuid =
                json::attribute_from_value(job, "uuid").expect("Unable to find uuid for job");
            let company_uuid = json::attribute_from_value(job, "company_uuid")
                .expect("Unable to find a company_uuid for this job.");
            let client = clients
                .iter()
                .find(|&client| json::attribute_from_value(client, "uuid").unwrap() == company_uuid)
                .expect("Unable to find the client related to this job");
            let job_contacts = contacts
                .iter()
                .filter(|&contact| {
                    json::attribute_from_value(contact, "job_uuid").unwrap() == job_uuid
                })
                .collect::<Vec<&Value>>();
            let job_activities = activities
                .iter()
                .filter(|&activity| {
                    json::attribute_from_value(activity, "job_uuid").unwrap() == job_uuid
                })
                .collect::<Vec<&Value>>();

            match opportunity_matches_job(opportunity, client, &job_contacts, &job_activities) {
                Ok(value) => value,
                Err(err) => {
                    println!("{}", err);
                    false
                }
            }
        })
        .is_some()
}