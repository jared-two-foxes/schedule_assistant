// name: email
// type: command line application
// desc: sends an email to the specified contact of each upcoming
//       servicem8 job within a specified window which outlines the
//       expected timelines for delivery and collection.

// Tasks
// [] parse the commandline arguments to get the query window
// [] pull the servicem8 jobs which fit within winodw
// [] iterate each of the jobs and compose the email to be sent
// [] send the emails

// Tasklist/Options?
// Pull the clients email out of servicem8
// if there is no email address dont bother attempting to send.
// add stmp server & login to the env file.

// Command Line Arguments
// Option to send all emails to jared@twofoxes.co.nz to check first
// download pickinglists from current and attach to emails.

use anyhow;
use chrono::prelude::*;
use handlebars::Handlebars;
use lettre::smtp::authentication::Credentials;
use lettre::{SmtpClient, Transport};
use lettre_email::{Email, EmailBuilder};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::{cmp, env};

use schedule_assistant::authentication::AuthenticationCache;
use schedule_assistant::{current_rms, json, servicem8};

fn calculate_window(
    activity: &Value,
    min_duration: chrono::Duration,
) -> (NaiveDateTime, chrono::Duration) {
    // Calculate the start time
    let start = Utc
        .datetime_from_str(
            activity["start_date"].as_str().unwrap(),
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap();
    let end = Utc
        .datetime_from_str(activity["end_date"].as_str().unwrap(), "%Y-%m-%d %H:%M:%S")
        .unwrap();
    let duration = round_up(end.time()) - round_down(start.time());

    (start.naive_local(), cmp::max(duration, min_duration))
}

fn round_up(time: NaiveTime) -> NaiveTime {
    let mut hour = time.hour();
    let mut minute = time.minute();
    if minute > 30 {
        minute = 0;
        hour += 1;
    } else {
        minute = 30;
    }
    NaiveTime::from_hms(hour, minute, 0)
}

fn round_down(time: NaiveTime) -> NaiveTime {
    let hour = time.hour();
    let mut minute = time.minute();
    if minute >= 30 {
        minute = 30;
    } else {
        minute = 0;
    }
    NaiveTime::from_hms(hour, minute, 0)
}

fn process_client_name(client: &Value) -> Option<Vec<String>> {
    let name = json::attribute_from_value(client, "name")?;
    let is_individual = client["is_individual"].as_u64().unwrap_or(1); //< Default to an individual.
    if is_individual == 1 {
        Some(split_name(name))
    } else {
        Some(vec![name])
    }
}

fn split_name(mut name: String) -> Vec<String> {
    let c = name.find(',');
    match c {
        // name is structured "lastname, firstname"
        Some(c) => {
            let second = name.drain(..c).collect();
            let first = name[2..].to_string();
            vec![first, second]
        }

        // name either has no surname, ie company name, or is structured
        // "firstname lastname"
        None => {
            let c = name.find(' ').unwrap_or_else(|| name.len());
            let first = name.drain(..c).collect();
            if !name.is_empty() {
                let second = name;
                vec![first, second]
            } else {
                vec![first]
            }
        }
    }
}

fn find_by_attribute<'a>(list: &'a Vec<Value>, attribute: &str, value: &str) -> Option<&'a Value> {
    list.iter()
        .find(|&company| company[attribute].as_str().unwrap() == value)
}

fn best_email(job: &Job, contacts: &Vec<Value>) -> Option<String> {
    // Grab the job contacts details.
    //@todo:  What are the potential values of this? ['JOB','BILLING'].  Does it make sense to send an email to
    //        the billing contact if we cant find the job contact?  Do we send it to everyone?  Extract all the
    //        available emails?
    let main_contact = contacts
        .iter()
        .filter(|&a| a["job_uuid"].as_str().unwrap() == job.uuid)
        .find(|&a| a["type"].as_str().unwrap() == "JOB")?;

    // Pull the email from the main contact.
    json::attribute_from_value(&main_contact, "email")
}

fn populate_email_data_from_job(
    job: &Job,
    companies: &Vec<Value>,
    activity_records: &Vec<Value>,
) -> Option<HashMap<String, String>> {
    // Sanitize and build client name.
    let client = find_by_attribute(&companies, "uuid", &job.company_uuid)?;
    let client_name = process_client_name(client)?;

    // Find all of the job_activities associated with this job.
    let _activities: Vec<(NaiveDateTime, chrono::Duration)> = activity_records
        .iter()
        .filter(|&a| a["job_uuid"].as_str().unwrap() == job.uuid)
        .map(|a| calculate_window(&a, chrono::Duration::hours(2)))
        .collect();

    // Populate the template substitution data.
    let mut data = HashMap::new();
    //@todo: Insert the activities as strings?
    // data.insert(
    //     "activites".to_string(),
    //     activities
    // );
    data.insert("job_address".to_string(), job.address.clone());
    data.insert("first_name".to_string(), client_name[0].clone());

    Some(data)
}

// Calculate the date window bounds;
// filter: Next week, starting from the following monday.
fn parse_command_line() -> (Date<Utc>, Date<Utc>) {
    let mut current = Utc::now();
    let weekday = current.weekday();
    let num_days = weekday.num_days_from_monday();
    if num_days > 1 {
        current = current + chrono::Duration::days((7 - num_days).into())
    }
    let start_of_week = current.date();
    let end_of_week = start_of_week + chrono::Duration::days(7);
    (start_of_week, end_of_week)
}

#[derive(Deserialize)]
struct Job {
    uuid: String,
    company_uuid: String,
    address: String,
}

fn query_relevant_jobs(
    auth_cache: &AuthenticationCache,
    start_of_week: Date<Utc>,
    end_of_week: Date<Utc>,
) -> reqwest::Result<Vec<Job>> {
    let activity_records: Vec<Value> = servicem8::job_activities(&auth_cache)?
        .into_iter()
        .filter(servicem8::activity_is_active)
        .collect::<Vec<Value>>();
    let jobs = servicem8::jobs(&auth_cache)?;

    // Aggregate all the job id's of the activities that fall in the active window.
    let mut job_ids: Vec<String> = activity_records
        .iter()
        .filter(|&a| {
            let start_date = json::date_from_value(a, "start_date", "%Y-%m-%d %H:%M:%S").unwrap();
            let job_date = start_date.date();
            job_date >= start_of_week && job_date < end_of_week
        })
        .map(|a| json::attribute_from_value(a, "job_uuid").unwrap())
        .collect();

    // Remove duplicates from the jobs_id list.
    job_ids.sort_unstable();
    job_ids.dedup();

    let found_jobs: Vec<Job> = jobs
        .into_iter()
        .filter(|j| match j["uuid"].as_str() {
            Some(uuid) => job_ids.iter().any(|id| id == uuid),
            None => false,
        })
        .map(|value| serde_json::from_value(value).unwrap())
        .collect();

    Ok(found_jobs)
}

fn populate_emails(
    auth_cache: &AuthenticationCache,
    jobs: &Vec<Job>,
) -> anyhow::Result<Vec<Email>> {
    let contacts = servicem8::job_contacts(&auth_cache)?;
    let companies = servicem8::clients(&auth_cache)?;
    let activity_records: Vec<Value> = servicem8::job_activities(&auth_cache)?; //< querying for these the second time, seems bad!
    let _opportunities = current_rms::opportunities(&auth_cache)?;
    // Setup email template engine.
    let handlebars = Handlebars::new();
    let mut source_template = File::open(&"./templates/template.hbs")?; //< If we cant find the template file, panic.
    let mut template_source = String::new();
    source_template.read_to_string(&mut template_source)?; //< Unable to parse template file, panic.

    let mut vec = Vec::new();
    for job in jobs {
        // Find the best email-address for the email.
        let email = match best_email(job, &contacts) {
            Some(data) => data,
            None => continue,
        };

        // Populate the template substitution data.
        let data = match populate_email_data_from_job(job, &companies, &activity_records) {
            Some(data) => data,
            None => continue,
        };

        // Create email html content
        let output = match handlebars.render_template(&template_source, &data) {
            Ok(data) => data,
            Err(e) => {
                println!("Unable to render email template: {}", e);
                continue;
            }
        };

        // Build the email
        println!("Building email");
        let email_builder = EmailBuilder::new()
            .from(("hello@twofoxes.co.nz", "Two Foxes"))
            .to(email) //< @todo: add an override for the delivery email for testing?
            .subject("Delivery Confirmation")
            .html(output); //< Not going to worry about non HTML email clients at this stage.  What is this the 90's?

        // // Find the opportunities for this job
        // // @todo: Maybe filter on the client name, email, phone number?
        // //       Then search for a job that is +/- 1 day of the foud job
        // println!("finding opportunity.");
        // if let Some(opportunity) = schedule_assistant::find_opportunity_for_job(
        //     &opportunities,
        //     client,
        //     &activities,
        //     &job_contacts,
        // ) {
        //     let opportunity_id = opportunity["id"].as_u64().unwrap();
        //     let (picking_list, filename) =
        //         current_rms::print_document_pdf(&auth_cache, opportunity_id, 4)?;
        //     let mut file = File::create(filename)?;
        //     file.write_all(&picking_list)?;

        //     // Attach to email.
        //     email_builder = email_builder
        //         .attachment(&data, &filename, &mime::APPLICATION_PDF)
        //         .unwrap();
        // };

        let email = email_builder.build()?;
        vec.push(email);
    }

    Ok(vec)
}

fn send_emails(emails: Vec<Email>) -> anyhow::Result<()> {
    // Setup the email sender.
    let smtp_address = env::var("SMTP_ADDRESS").expect("SMTP_ADDRESS not found");
    let smtp_username = env::var("SMTP_USERNAME").expect("SMTP_USERNAME not found");
    let smtp_password = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not found");
    let mut mailer =
        SmtpClient::new_simple(&smtp_address)? //< if this fails just bail since we cant do anything that we need to.`
            .credentials(Credentials::new(smtp_username.into(), smtp_password.into()))
            .transport();

    for email in emails {
        mailer.send(email.into())?;
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().expect("Failed to read .env file");

    let (start_of_week, end_of_week) = parse_command_line();
    let auth_cache = AuthenticationCache::new();
    let jobs = query_relevant_jobs(&auth_cache, start_of_week, end_of_week)?;
    let emails = populate_emails(&auth_cache, &jobs)?;
    send_emails(emails)
}
