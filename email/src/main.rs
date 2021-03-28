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
use lettre_email::EmailBuilder;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::prelude::*;

use schedule_assistant::authentication::AuthenticationCache;
use schedule_assistant::{json, servicem8};

fn calculate_window(
    activity: &Value,
    max_duration: chrono::Duration,
) -> (NaiveDate, NaiveTime, NaiveTime) {
    // Calculate the start time
    let start = Utc
        .datetime_from_str(
            activity["start_date"].as_str().unwrap(),
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap();
    let start_time = start.time();

    let end = Utc
        .datetime_from_str(activity["end_date"].as_str().unwrap(), "%Y-%m-%d %H:%M:%S")
        .unwrap();
    let max_end_time = start_time + max_duration;
    let mut end_time = end.time();
    if end_time < max_end_time {
        end_time = max_end_time;
    }

    (
        start.naive_local().date(),
        round_down(start_time),
        round_up(end_time),
    )
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

fn best_email(job: &Value, contacts: &Vec<Value>) -> Option<String> {
    let job_uuid = json::attribute_from_value(job, "uuid")?;

    // Get associated contacts to this job.
    let job_contacts: Vec<&Value> = contacts
        .iter()
        .filter(|&a| a["job_uuid"].as_str().unwrap() == job_uuid)
        .collect();

    // Grab the job contacts details.
    //@todo:  What are the potential values of this? ['JOB','BILLING'].  Does it make sense to send an email to
    //        the billing contact if we cant find the job contact?  Do we send it to everyone?  Extract all the
    //        available emails?
    let main_contact = job_contacts
        .iter()
        .find(|&a| a["type"].as_str().unwrap() == "JOB")?;

    // Pull the email from the main contact.
    json::attribute_from_value(&main_contact, "email")
}

fn populate_email_data_from_job(
    job: &Value,
    companies: &Vec<Value>,
    activity_records: &Vec<Value>,
) -> Option<HashMap<String, String>> {
    let job_uuid = json::attribute_from_value(job, "uuid")?;
    let company_uuid = json::attribute_from_value(job, "company_uuid")?;
    let job_address = json::attribute_from_value(job, "job_address")?;

    // Sanitize and build client name.
    let client = find_by_attribute(&companies, "uuid", &company_uuid)?;
    let client_name = process_client_name(client)?;

    // Find all of the job_activities associated with this job.
    let activities: Vec<&serde_json::Value> = activity_records
        .iter()
        .filter(|&a| a["job_uuid"].as_str().unwrap() == job_uuid)
        .collect();

    // Determine the delivery times.
    let (delivery_date, delivery_start, delivery_end) =
        calculate_window(activities[0], chrono::Duration::hours(2));

    // Determine the collection times.
    let (collection_date, collection_start, collection_end) =
        calculate_window(activities[1], chrono::Duration::hours(2));
    // Populate the template substitution data.
    let mut data = HashMap::new();
    data.insert(
        "delivery_date".to_string(),
        delivery_date.format("%a, %e %h").to_string(),
    );
    data.insert(
        "delivery_start_time".to_string(),
        delivery_start.format("%l:%M %P").to_string(),
    );
    data.insert(
        "delivery_end_time".to_string(),
        delivery_end.format("%l:%M %P").to_string(),
    );
    data.insert(
        "collection_date".to_string(),
        collection_date.format("%a, %e %h").to_string(),
    );
    data.insert(
        "collection_start_time".to_string(),
        collection_start.format("%l:%M %P").to_string(),
    );
    data.insert(
        "collection_end_time".to_string(),
        collection_end.format("%l:%M %P").to_string(),
    );
    data.insert("job_address".to_string(), job_address);
    data.insert("first_name".to_string(), client_name[0].clone());

    Some(data)
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().expect("Failed to read .env file");

//
// Process commandline arguments
//
    // Calculate the date filter; Next week, starting from the following monday.
    let mut current = Utc::now();
    let weekday = current.weekday();
    let num_days = weekday.num_days_from_monday();
    if num_days > 1 {
        current = current + chrono::Duration::days((7 - num_days).into())
    }
    let start_of_week = current.date();
    let end_of_week = start_of_week + chrono::Duration::days(7);

//
// Retrieve all relevant data required to process.
//
    let auth_cache = AuthenticationCache::new();
    let activity_records: Vec<Value> = servicem8::job_activities(&auth_cache)?
        .into_iter()
        .filter(servicem8::activity_is_active)
        .collect::<Vec<Value>>();
    let jobs = servicem8::jobs(&auth_cache)?;
    let contacts = servicem8::job_contacts(&auth_cache)?;
    let companies = servicem8::clients(&auth_cache)?;
    //let _opportunities = current_rms::opportunities(&auth_cache)?;

//
// Setup email template engine.
//
    let handlebars = Handlebars::new();
    let mut source_template = File::open(&"./templates/template.hbs")?; //< If we cant find the template file, panic.
    let mut template_source = String::new();
    source_template.read_to_string(&mut template_source)?; //< Unable to parse template file, panic.

//
// Setup the email sender.
//
    let smtp_address = env::var("SMTP_ADDRESS").expect("SMTP_ADDRESS not found");
    let smtp_username = env::var("SMTP_USERNAME").expect("SMTP_USERNAME not found");
    let smtp_password = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not found");
    let mut mailer =
        SmtpClient::new_simple(&smtp_address)? //< if this fails just bail since we cant do anything that we need to.`
            .credentials(Credentials::new(smtp_username.into(), smtp_password.into()))
            .transport();    

//
// Find all the jobs requiring emails to be sent.
//
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

    let found_jobs: Vec<Value> = jobs
        .into_iter()
        .filter(|j| match j["uuid"].as_str() {
            Some(uuid) => job_ids.iter().any(|id| id == uuid),
            None => false,
        })
        .collect();

//
// Send email to each job requiring it.
//
    // Iterate all the jobs that we've found, and send them schedule information.
    for job in &found_jobs {
        // Find the best email-address for the email.
        let email = match best_email(&job, &contacts) {
            Some(data) => data,
            None => continue,
        };

        // Populate the template substitution data.
        let data = match populate_email_data_from_job(&job, &companies, &activity_records) {
            Some(data) => data,
            None => continue,
        };

        // Create email html content
        let output = match handlebars.render_template(&template_source, &data) {
            Ok(data) => data, 
            Err(e) => {
                println!("Unable to render email template: {}", e);
                continue
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

        // Send the email!
        println!("Sending email");
        match email_builder.build() {
            Ok(email) => {
                let result = mailer.send(email.into());
                println!("{:?}", result);
            }
            Err(e) => {
                println!("Error sending email: {}", e);
            }
        };
    }

    Ok(())
}
