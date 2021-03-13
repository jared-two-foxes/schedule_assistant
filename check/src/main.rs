// Tasklist/Options?
// Pull the clients email out of servicem8
// if there is no email address dont bother attempting to send.
// add stmp server & login to the env file.

// Command Line Arguments
// Option to send all emails to jared@twofoxes.co.nz to check first
// download pickinglists from current and attach to emails.

//@todo:
// Need to restrict the opportunities to those that are confirmed.




use anyhow;
use chrono::prelude::*;
use reqwest;
use serde_json::Value;

use schedule_assistant::{current_rms, guard, json, servicem8};

fn date_bound_by(date: &Date<Utc>, start: &Date<Utc>, end: &Date<Utc>) -> bool {
    date > start && date < end
}

type Period<T> = (Date<T>, Date<T>);

fn period_bound_by(period: &Period<Utc>, start: &Date<Utc>, end: &Date<Utc>) -> bool {
    date_bound_by(&period.0, start, end) && date_bound_by(&period.1, start, end)
}

fn activity_within_date_range(
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

fn opportunity_within_date_range(op: &Value, start: &Date<Utc>, end: &Date<Utc>) -> Option<bool> {
    let starts_at = json::date_from_value(op, "starts_at", "%Y-%m-%dT%H:%M:%S%.3f%Z")?;
    let ends_at = json::date_from_value(op, "ends_at", "%Y-%m-%dT%H:%M:%S%.3f%Z")?;
    Some(period_bound_by(
        &(starts_at.date(), ends_at.date()),
        start,
        end,
    ))
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

fn opportunity_has_job(
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

fn calculate_date_range() -> (Date<Utc>, Date<Utc>) {
    let mut current = Utc::now();
    let weekday = current.weekday();
    let num_days = weekday.num_days_from_monday();
    if num_days > 0 {
        current = current + chrono::Duration::days((7 - num_days).into())
    }
    let start_of_week = current.date();
    let end_of_week = start_of_week + chrono::Duration::days(7);
    (start_of_week, end_of_week)
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().expect("Failed to read .env file");
    let (start_of_week, end_of_week) = calculate_date_range(); //< Calculate the date filter; Next week, starting from the following monday.

    // Get Current-RMS opportunities.
    let opportunities = current_rms::opportunities()?
        .into_iter()
        .filter(|opportunity| {
            opportunity_within_date_range(opportunity, &start_of_week, &end_of_week).unwrap()
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
        .iter()
        .filter(|&o| !opportunity_has_job(o, &jobs, &clients, &job_contacts, &job_activities))
        .collect::<Vec<&Value>>();

    println!(
        "There are {} unscheduled jobs for the week {} to {}",
        unscheduled.len(),
        start_of_week,
        end_of_week
    );

    for opportunity in unscheduled {
        let member = &opportunity["member"];
        let member_name = json::attribute_from_value(&member, "name").expect("Unable to grab client name");
        println!("{}", member_name);
    }

    Ok(())
}
