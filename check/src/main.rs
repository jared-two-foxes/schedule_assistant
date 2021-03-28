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
use schedule_assistant::json;
use std::env;

fn calculate_date_range(date: DateTime<Utc>) -> (Date<Utc>, Date<Utc>) {
    let mut current = date;
    let weekday = date.weekday();
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

    let auth_cache = schedule_assistant::authentication::AuthenticationCache::new();
    let args: Vec<String> = env::args().collect();
    let start = if args.len() > 1 { 
        Utc.datetime_from_str(&args[1], "%Y-%m-%d %H:%M:%S")? //"2021-03-22 00:00:00"
    }
    else {
        Utc::now()
    };

    
    let (start_of_week, end_of_week) = calculate_date_range(start); //< Calculate the date filter; Next week, starting from the following monday.
    let unscheduled = schedule_assistant::check_bookings(&auth_cache, start_of_week, end_of_week)?;

    println!(
        "There are {} unscheduled opportunities for the week {} to {}",
        unscheduled.len(),
        start_of_week,
        end_of_week
    );

    for opportunity in unscheduled {
        let member = &opportunity["member"];
        let member_name = json::attribute_from_value(&member, "name")
            .expect("Unable to extract client name from opportunity");
        let opportunity_id = opportunity["id"].as_u64().unwrap();
        println!("{} ({})", member_name, opportunity_id);
    }

    Ok(())
}
