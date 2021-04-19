pub mod authentication;
pub mod database;
mod endpoints;
pub mod json;
mod macros;
mod retrieve;

pub mod current_rms;
pub mod servicem8;

use authentication::AuthenticationCache;
use chrono::{Date, DateTime, Utc};
use serde_json::Value;

fn date_bound_by(date: &Date<Utc>, start: &Date<Utc>, end: &Date<Utc>) -> bool {
    date > start && date < end
}

type Period<T> = (Date<T>, Date<T>);
fn period_bound_by(period: &Period<Utc>, start: &Date<Utc>, end: &Date<Utc>) -> bool {
    date_bound_by(&period.0, start, end) && date_bound_by(&period.1, start, end)
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

    let mut name = format!("{} {}", first_name, last_name).to_string();
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

pub fn opportunity_within_date_range(
    op: &Value,
    start: &Date<Utc>,
    end: &Date<Utc>,
) -> Option<bool> {
    let starts_at = json::date_from_value(op, "starts_at", "%Y-%m-%dT%H:%M:%S%.3f%Z")?;
    let ends_at = json::date_from_value(op, "ends_at", "%Y-%m-%dT%H:%M:%S%.3f%Z")?;
    Some(period_bound_by(
        &(starts_at.date(), ends_at.date()),
        start,
        end,
    ))
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

    let member_matches = member_matches_client(member, &client);
    let contact_matches = job_contacts
        .iter()
        .any(|&contact| member_matches_contact(member, &contact));
    if member_matches || contact_matches {
        let starts_at =
            json::date_from_value(opportunity, "starts_at", "%Y-%m-%dT%H:%M:%S%.3f%Z").unwrap();
        let ends_at =
            json::date_from_value(opportunity, "ends_at", "%Y-%m-%dT%H:%M:%S%.3f%Z").unwrap();
        for activity in job_activities {
            if let Some(value) =
                activity_within_date_range(&activity, &starts_at.date(), &ends_at.date())
            {
                if value {
                    return Ok(true);
                }
            }
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

pub fn find_opportunity_for_job<'a>(
    opportunities: &'a Vec<Value>,
    client: &Value,
    job_contacts: &Vec<&Value>,
    job_activities: &Vec<&Value>,
) -> Option<&'a Value> {
    opportunities.iter().find(|&opportunity| {
        opportunity_matches_job(opportunity, client, job_contacts, job_activities).unwrap_or(false)
    })
}

pub fn check_bookings(
    auth_cache: &AuthenticationCache,
    start: Date<Utc>,
    end: Date<Utc>,
) -> reqwest::Result<Vec<Value>> {
    // Panic's if we cant open the database.
    let conn = database::connection().unwrap(); 

    // PANIC:  Will panic if table is already created.
    database::create_table(&conn).unwrap();
    // PANIC:  Will panic if the person already exists in the table.
    database::insert(&conn).unwrap();
    // PANIC: If we are unable to query the database.
    let people = database::query(&conn).unwrap();
    for person in people {
        println!("Found person {:?}", person);
    }

    // Get Current-RMS opportunities.
    let opportunities = current_rms::opportunities(auth_cache)?
        .into_iter()
        .filter(|opportunity| {
            current_rms::opportunity_is_confirmed(opportunity)
                && opportunity_within_date_range(opportunity, &start, &end).unwrap()
        })
        .collect::<Vec<Value>>();

    // Get ServiceM8 data that we're going to need.
    let jobs = servicem8::jobs(auth_cache)?;
    let clients = servicem8::clients(auth_cache)?;
    let job_activities = servicem8::job_activities(auth_cache)?;
    let job_contacts = servicem8::job_contacts(auth_cache)?;

    // Check that all the opportunities have jobs registered in servicem8 with allocated
    // Activities for delivery & collection.
    let unscheduled = opportunities
        .into_iter()
        .filter(|opportunity| {
            !opportunity_has_job(
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

pub fn remove_expired_quotes(auth_cache: &AuthenticationCache, date: DateTime<Utc>) {
    // get all the opportunities
    let opportunities = current_rms::opportunities(auth_cache).unwrap();

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
                        match current_rms::mark_as_lost(auth_cache, opportunity_id) {
                            Err(e) => println!("Err: Failed to mark the event as lost. {}", e),
                            _ => println!("Marked event as lost."),
                        }
                    }
                }
                None => {}
            }
        });
}
