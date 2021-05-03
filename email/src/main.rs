// name: email
// type: command line application
// desc: sends an email to the specified contact of each upcoming
//       servicem8 job within a specified window which outlines the
//       expected timelines for delivery and collection.

// Tasks
// [x] parse the commandline arguments to get the query window
// [x] pull the servicem8 jobs which fit within winodw
// [] compose the emails to be sent
// [] attach the picking lists
// [x] send the emails


//@todo: Update the handlebars rendering to use the changes added by the json data structure change.
//@todo: Filter the opportunities by active when requesting.  No point getting old opportunities

use anyhow;
use chrono::prelude::*;
use chrono::{Duration, DurationRound};
use handlebars::Handlebars;
use lettre::smtp::authentication::Credentials;
use lettre::{SmtpClient, Transport};
use lettre_email::{Email, EmailBuilder};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs::File;
use std::io::prelude::*;
use std::{cmp, env};

use schedule_assistant::authentication::AuthenticationCache;
use schedule_assistant::{current_rms, json, servicem8};

// servicem8 uses a date format '%Y-%m-%d %H:%M:%S' which while DateTime
// supports Serde out of the box, it uses the RFC3339 format so we need
// to provide some custom logic to help it understand how to deserialize
// our desired format.
mod my_date_format {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize)]
struct JobActivity {
    uuid: String,
    job_uuid: String,
    #[serde(with = "my_date_format")]
    start_date: DateTime<Utc>, //< "%Y-%m-%d %H:%M:%S" format
    #[serde(with = "my_date_format")]
    end_date: DateTime<Utc>, //< "%Y-%m-%d %H:%M:%S" format
}

fn calculate_window(
    activity: &JobActivity,
    min_duration: Duration,
) -> (NaiveDateTime, chrono::Duration) {
    let end_time = activity
        .end_date
        .duration_round(Duration::minutes(30))
        .unwrap()
        .time();
    let start_time = activity
        .start_date
        .duration_trunc(Duration::minutes(30))
        .unwrap()
        .time();
    let duration = end_time - start_time;

    (
        activity.start_date.naive_local(),
        cmp::max(duration, min_duration),
    )
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
    activity_records: &Vec<JobActivity>,
) -> Option<serde_json::Value> {
    // Sanitize and build client name.
    let client = find_by_attribute(&companies, "uuid", &job.company_uuid)?;
    let client_name = process_client_name(client)?;

    // Find all of the job_activities associated with this job.
    let activities: Vec<(NaiveDateTime, NaiveDateTime)> = activity_records
        .iter()
        .filter(|&a| a.job_uuid == job.uuid)
        .map(|a| calculate_window(&a, chrono::Duration::hours(2))) //< (start, duration)
        .map(|i| (i.0, i.0 + i.1)) //< (start, end)
        .collect();

    // Populate the template substitution data.
    let data = json!({
        "job_address": job.job_address,
        "first_name": client_name[0],
        "activities": activities
    });

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
    job_address: String,
}

fn query_relevant_jobs(
    auth_cache: &AuthenticationCache,
    start_of_week: Date<Utc>,
    end_of_week: Date<Utc>,
) -> reqwest::Result<Vec<Job>> {
    let activity_records = servicem8::job_activities(&auth_cache)?
        .into_iter()
        .filter(servicem8::activity_is_active)
        .map(|a| serde_json::from_value(a).unwrap())
        .collect::<Vec<JobActivity>>();
    let jobs = servicem8::jobs(&auth_cache)?;

    // Aggregate all the job id's of the activities that fall in the active window.
    let mut job_ids: Vec<String> = activity_records
        .iter()
        .filter(|&a| {
            let job_date = a.start_date.date();
            job_date >= start_of_week && job_date < end_of_week
        })
        .map(|a| a.job_uuid.clone())
        .collect();

    // Remove duplicates from the jobs_id list.
    job_ids.sort_unstable();
    job_ids.dedup();
    let found_jobs: Vec<Job> = jobs
        .into_iter()
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
    let activity_records = servicem8::job_activities(&auth_cache)?
        .into_iter()
        .map(|a| serde_json::from_value(a).unwrap())
        .collect::<Vec<JobActivity>>(); //< querying for these the second time, seems bad!
    let _opportunities = current_rms::opportunities(&auth_cache)?;

    // Setup email template engine.
    let handlebars = Handlebars::new();
    let mut source_template = File::open(&"./templates/template.hbs")?; //< If we cant find the template file, panic.
    let mut template_source = String::new();
    source_template.read_to_string(&mut template_source)?; //< Unable to parse template file, panic.

    let mut vec = Vec::new();
    for job in jobs {
        // Find the best email-address for the email.
        let email_address = match best_email(job, &contacts) {
            Some(data) => data,
            None => {
                println!("Unable to find email address for job");
                continue;
            }
        };

        // Populate the template substitution data.
        let data = match populate_email_data_from_job(job, &companies, &activity_records) {
            Some(data) => data,
            None => {
                println!("Unable to populate email data for job.");
                continue;
            }
        };

        // Create email html content
        let output = match handlebars.render_template(&template_source, &data) {
            Ok(data) => data,
            Err(e) => {
                println!("Unable to render email template: {}", e);
                continue;
            }
        };
        
        println!("{}", output);

        // Build the email
        let email_builder = EmailBuilder::new()
            .from(("hello@twofoxes.co.nz", "Two Foxes"))
            .to(email_address) //< @todo: add an override for the delivery email for testing?
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

        println!("Building email");
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
