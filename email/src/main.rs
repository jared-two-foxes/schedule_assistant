// Tasklist/Options?
// Pull the clients email out of servicem8
// if there is no email address dont bother attempting to send.
// add stmp server & login to the env file.

// Command Line Arguments
// Option to send all emails to jared@twofoxes.co.nz to check first
// download pickinglists from current and attach to emails.

use chrono::prelude::*;
use handlebars::Handlebars;
use lettre::smtp::authentication::Credentials;
use lettre::{SmtpClient, Transport};
use lettre_email::{mime, EmailBuilder};
use serde_json::Value;
use std::collections::HashMap;
use std::convert::TryInto;
use std::env;
use std::fs::File;
use std::io::prelude::*;

pub fn activities(filter: Option<&str>) -> Vec<Value> {
    let mut output = Vec::new();
    let result = servicem8::get_job_activities(filter);
    match result {
        Err(err) => println!("{}", err),
        Ok(response) => {
            if response.is_array() {
                let jobs = response.as_array().unwrap();
                output.extend(jobs.clone().into_iter());
            } else {
                output.push(response);
            }
        }
    };

    output
}

pub fn jobs(filter: Option<&str>) -> Vec<Value> {
    let mut output = Vec::new();
    let result = servicem8::get_jobs(filter);
    match result {
        Err(err) => println!("{}", err),
        Ok(response) => {
            if response.is_array() {
                // Cloning all the objects found and push all these structures into a vector to
                // be returned.
                let jobs = response.as_array().unwrap();
                output.extend(jobs.clone().into_iter());
            } else {
                output.push(response);
            }
        }
    };
    output
}

pub fn opportunities() -> Vec<Value> {
    let mut page = 0;
    let per_page = 50u32;
    let mut output = Vec::new();
    loop {
        page += 1;

        let result = currentrms::get_opportunities(&page, &per_page);
        match result {
            Err(err) => {
                println!("{}", err);
                break;
            }
            Ok(response) => {
                let opportunities_object = &response["opportunities"];
                if !opportunities_object.is_array() {
                    break;
                }

                // If there are no more opportunities then we're done.
                let opportunities = opportunities_object.as_array().unwrap();
                if opportunities.is_empty() {
                    break;
                }
                // Cloning all the objects found and push all these structures into a vector to
                // be returned.
                output.extend(opportunities.clone().into_iter());
            }
        };
    }
    output
}

pub fn opportunities_documents(opportunity_id: u32) -> Vec<Value> {
    let mut page = 0;
    let per_page = 50u32;
    let mut output = Vec::new();
    loop {
        page += 1;

        let result = currentrms::get_opportunity_documents(page, per_page, opportunity_id);
        match result {
            Err(err) => {
                println!("{}", err);
                break;
            }
            Ok(response) => {
                let array_object = &response["opportunity_documents"];
                if !array_object.is_array() {
                    break;
                }
                // If there are no more opportunities then we're done.
                let items = array_object.as_array().unwrap();
                if items.is_empty() {
                    break;
                }
                // Cloning all the objects found and push all these structures into a vector to
                // be returned.
                output.extend(items.clone().into_iter());
            }
        };
    }
    output
}

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

fn raw_name(client: &Value) -> String {
    client["name"].as_str().unwrap().to_string()
}

fn split_name(client: &Value) -> (String, String) {
    let mut first_name = String::from("");
    let mut last_name = String::from("");
    let mut name = raw_name(client);
    if let Some(is_individual) = client["is_individual"].as_i64() {
        if is_individual == 1 {
            let c = name.find(',');
            match c {
                // client name is structured "lastname, firstname"
                Some(c) => {
                    last_name = name.drain(..c).collect();
                    first_name = name[2..].to_string();
                }
                // client name either has no surname or is structured
                // "firstname lastname"
                None => {
                    let c = name.find(' ').unwrap_or_else(|| name.len());
                    first_name = name.drain(..c).collect();
                    if !name.is_empty() {
                        last_name = name;
                    }
                }
            }
        } else {
            first_name = name;
        }
    }
    (first_name, last_name)
}

fn full_name(client: &Value) -> String {
    let mut name = raw_name(client);
    if let Some(is_individual) = client["is_individual"].as_i64() {
        if is_individual == 1 {
            let (first, last) = split_name(client);
            name = format!("{} {}", first, last)
        }
    }
    name
}

fn main() {
    dotenv::dotenv().expect("Failed to read .env file");

    // Calculate the date filter; Next week, starting from the following monday.
    let mut current = Utc::now();
    let weekday = current.weekday();
    let num_days = weekday.num_days_from_monday();
    if num_days > 1 {
        current = current + chrono::Duration::days((7 - num_days).into())
    }
    let start_of_week = current.date();
    let end_of_week = start_of_week + chrono::Duration::days(7);

    // Aggregate all the job id's of the activities that fall in the active window.
    let activity_records: Vec<Value> = activities(Some("active eq '1'"));
    let mut job_ids: Vec<&str> = activity_records
        .iter()
        .filter(|&a| {
            let start_date_str = a["start_date"].as_str().unwrap();
            let start_date = Utc
                .datetime_from_str(start_date_str, "%Y-%m-%d %H:%M:%S")
                .unwrap();
            let job_date = start_date.date();
            job_date >= start_of_week && job_date < end_of_week
        })
        .map(|a| a["job_uuid"].as_str().unwrap())
        .collect();

    // Remove duplicates from the jobs_id list.
    job_ids.sort_unstable();
    job_ids.dedup();

    // Grab the job objects
    let jobs: Vec<Value> = jobs(None)
        .into_iter()
        .filter(|j| match j["uuid"].as_str() {
            Some(uuid) => job_ids.iter().any(|&id| id == uuid),
            None => false,
        })
        .collect();

    // Setup email templating.
    let handlebars = Handlebars::new();
    let mut source_template = File::open(&"./templates/template.hbs").unwrap();
    let mut template_source = String::new();
    source_template
        .read_to_string(&mut template_source)
        .unwrap();

    // Setup the email sender.
    let smtp_address = env::var("SMTP_ADDRESS").expect("SMTP_ADDRESS not found");
    let smtp_username = env::var("SMTP_USERNAME").expect("SMTP_USERNAME not found");
    let smtp_password = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not found");
    let mut mailer = SmtpClient::new_simple(&smtp_address)
        .unwrap()
        .credentials(Credentials::new(smtp_username.into(), smtp_password.into()))
        .transport();

    // Grab all of the opportunities.
    let opportunities = opportunities();
    // Iterate all the jobs that we've found, and send them schedule information.
    for j in &jobs {
        let job_uuid = j["uuid"].as_str().unwrap();

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

        // Get clients name
        let client = servicem8::get_client(j["company_uuid"].as_str().unwrap()).unwrap();
        let full_name = full_name(&client);
        let client_name = split_name(&client).0;

        println!("Processing job for {}", full_name);

        // Populate the template substitution data.
        let mut data = HashMap::new();
        data.insert(
            "delivery_date",
            delivery_date.format("%a, %e %h").to_string(),
        );
        data.insert(
            "delivery_start_time",
            delivery_start.format("%l:%M %P").to_string(),
        );
        data.insert(
            "delivery_end_time",
            delivery_end.format("%l:%M %P").to_string(),
        );
        data.insert(
            "collection_date",
            collection_date.format("%a, %e %h").to_string(),
        );
        data.insert(
            "collection_start_time",
            collection_start.format("%l:%M %P").to_string(),
        );
        data.insert(
            "collection_end_time",
            collection_end.format("%l:%M %P").to_string(),
        );
        data.insert(
            "job_address",
            j["job_address"].as_str().unwrap().to_string(),
        );
        data.insert("first_name", client_name.to_string());

        // Create email html content
        let output = handlebars.render_template(&template_source, &data).unwrap();

        // Build the email
        //println!("Building email");
        let mut email_builder = EmailBuilder::new()
            .from(("hello@twofoxes.co.nz", "Two Foxes"))
            .to(("jared@twofoxes.co.nz", "Jared Watt")) //@todo: Need to grab email from clients record.
            .subject("Delivery Confirmation")
            .alternative(output, "this is the backup data if html is not supported?");

        // Find the opportunities for this job
        //@todo: Maybe filter on the client name, email, phone number?
        //       Then search for a job that is +/- 1 day of the found job
        //println!("finding opportunity.");
        let it = opportunities.iter().find(|&o| {
            let member = &o["member"];
            if !member.is_object() {
                return false;
            }
            if !member["name"].is_string() {
                return false;
            }
            let owner_name = member["name"].as_str().unwrap();
            if owner_name == full_name {
                return true;
            }
            //@todo: filter on email?
            false
        });

        match it {
            Some(opportunity) => {
                println!("Found Opportunity");
                let opportuntity_id = opportunity["id"].as_i64().unwrap().try_into().unwrap();

                // Grab all the documents associated with this opportunity
                let opportunity_documents = opportunities_documents(opportuntity_id);
                // Find the pickinglist document
                let it = opportunity_documents
                    .iter()
                    .find(|&d| d["document_id"] == 4);
                let (data, filename) = match it {
                    Some(d) => {
                        // Download the pickinglist
                        currentrms::get_opportunity_document_pdf(
                            d["id"].as_i64().unwrap().try_into().unwrap(),
                        )
                        .unwrap()
                    }
                    None => {
                        //@todo: Attempt to create the document...?
                        println!("Unable to find document, attempting to prompt its creation.");
                        currentrms::print_opportunity_document_pdf(opportuntity_id, 4).unwrap()
                    }
                };

                // Attach to email.
                email_builder = email_builder
                    .attachment(&data, &filename, &mime::APPLICATION_PDF)
                    .unwrap();
            }
            _ => println!("Unable to find opportunity"),
        }

        // Send the email!
        println!("Sending email");
        let email = email_builder.build().unwrap();
        let result = mailer.send(email.into());
        println!("{:?}", result);
    }
}
