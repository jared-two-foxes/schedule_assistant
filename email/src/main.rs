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

use schedule_assistant::{extract_or_continue, json, servicem8};

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

fn split_name(client: &Value) -> Option<(String, String)> {
    let mut first_name = String::from("");
    let mut last_name = String::from("");
    let mut name = json::attribute_from_value(client, "name")?;
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
    Some((first_name, last_name))
}

fn main() -> anyhow::Result<()> {
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
    let activity_records: Vec<Value> = servicem8::job_activities()?
        .into_iter()
        .filter(servicem8::activity_is_active)
        .collect::<Vec<Value>>();
    let mut job_ids: Vec<String> = activity_records
        .iter()
        .filter(|&a| {
            let start_date_str = json::attribute_from_value(a, "start_date").unwrap();
            let start_date = Utc
                .datetime_from_str(&start_date_str, "%Y-%m-%d %H:%M:%S")
                .unwrap();
            let job_date = start_date.date();
            job_date >= start_of_week && job_date < end_of_week
        })
        .map(|a| json::attribute_from_value(a, "job_uuid").unwrap())
        .collect();

    // Remove duplicates from the jobs_id list.
    job_ids.sort_unstable();
    job_ids.dedup();

    // Grab the job objects
    let jobs: Vec<Value> = servicem8::jobs()?
        .into_iter()
        .filter(|j| match j["uuid"].as_str() {
            Some(uuid) => job_ids.iter().any(|id| id == uuid),
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

    // Grab lists of structures of interest.
    //let opportunities = currentrms::opportunities(&currentrms_endpoint)?;
    let contacts = servicem8::job_contacts()?;
    let companies = servicem8::clients()?;

    // Iterate all the jobs that we've found, and send them schedule information.
    for j in &jobs {
        let job_uuid = extract_or_continue!(j, "uuid");
        let company_uuid = extract_or_continue!(j, "company_uuid");

        // Get associated contacts to this job.
        let job_contacts: Vec<&Value> = contacts
            .iter()
            .filter(|&a| a["job_uuid"].as_str().unwrap() == job_uuid)
            .collect();

        // Grab the billing contact.
        let billing_contact = job_contacts
            .iter()
            .filter(|&a| a["type"].as_str().unwrap() == "BILLING")
            .next();

        // Client
        let client = match companies
            .iter()
            .filter(|&company| company["uuid"].as_str().unwrap() == company_uuid)
            .next()
        {
            Some(client) => client,
            None => continue,
        };

        let raw_name = json::attribute_from_value(&client, "name").unwrap();
        println!("Processing job for {}", raw_name);

        // Grab the Clients contact details, we may need them later.
        let client_details = match split_name(&client) {
            Some(details) => details,
            None => {
                println!("Unable to extract client details for job {}", job_uuid);
                continue;
            }
        };

        // Get Job Contact information
        let (first_name, _last_name, email) = match billing_contact {
            Some(contact) => {
                let first_name =
                    json::attribute_from_value(&contact, "first").unwrap_or(client_details.0);
                let last_name =
                    json::attribute_from_value(&contact, "last").unwrap_or(client_details.1);
                let email =
                    json::attribute_from_value(&contact, "email").unwrap_or(String::from(""));
                (first_name, last_name, email)
            }
            None => {
                println!("Unable to find billing contact for job");
                (client_details.0, client_details.1, String::from(""))
            }
        };

        // If we dont have an email we cant really do anything here so skip to the next.
        //@todo:  Do we try to send an email to the other contact?
        if email == "" {
            println!("Unable to find an email for job, continuing");
            continue;
        }

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
        data.insert("first_name", first_name.to_string());

        // Create email html content
        let output = handlebars.render_template(&template_source, &data).unwrap();

        // Find the opportunities for this job
        //@todo: Maybe filter on the client name, email, phone number?
        //       Then search for a job that is +/- 1 day of the found job
        // println!("finding opportunity.");
        // let full_name = format!("{} {}", first_name, last_name).trim().to_string();
        // let it = opportunities.iter().find(|&o| {
        //     let member = &o["member"];
        //     if !member.is_object() {
        //         return false;
        //     }
        //     if let Some(value) = member["name"].as_str() {
        //         if value.trim() == full_name {
        //             return true;
        //         }
        //     }
        //     if let Some(value) = member["email"].as_str() {
        //         if value.trim() == email {
        //             return true;
        //         }
        //     }

        //     false
        // });

        // Build the email
        //println!("Building email");
        let /*mut*/ email_builder = EmailBuilder::new()
            .from(("hello@twofoxes.co.nz", "Two Foxes"))
            .to(email) //< @todo: add an override for the delivery email for testing?
            .subject("Delivery Confirmation")
            .alternative(output, "this is the backup data if html is not supported?");

        // match it {
        //     Some(opportunity) => {
        //         println!("Found Opportunity");
        //         let opportuntity_id = opportunity["id"].as_i64().unwrap();

        //         // Grab all the documents associated with this opportunity
        //         let params = [format!("opportunity_id={}", opportuntity_id)];
        //         let _opportunity_documents = currentrms::retrieve_paged(
        //             &currentrms_endpoint,
        //             currentrms::Object::OpportunityDocuments,
        //             Some(&params),
        //         )
        //         .unwrap();

        //         // Find the pickinglist document
        //         let it = opportunity_documents
        //             .iter()
        //             .find(|&d| d["document_id"] == 4);
        //         let (data, filename) = match it {
        //             Some(d) => {
        //                 // Download the pickinglist
        //                 currentrms::get_opportunity_document_pdf(
        //                     d["id"].as_i64().unwrap().try_into().unwrap(),
        //                 )
        //                 .unwrap()
        //             }
        //             None => {
        //                 // Attempt to create the document...?
        //                 println!("Unable to find document, attempting to prompt its creation.");
        //                 currentrms::print_opportunity_document_pdf(opportuntity_id, 4).unwrap()
        //             }
        //         };

        //         // Attach to email.
        //         email_builder = email_builder
        //             .attachment(&data, &filename, &mime::APPLICATION_PDF)
        //             .unwrap();
        //     }
        //     _ => println!("Unable to find opportunity"),
        // }

        // Send the email!
        println!("Sending email");
        let email = email_builder.build().unwrap();
        let result = mailer.send(email.into());
        println!("{:?}", result);
    }

    Ok(())
}
