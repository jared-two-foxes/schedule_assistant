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
    let unscheduled = schedule_assistant::check_bookings(start_of_week, end_of_week)?;

    println!(
        "There are {} unscheduled jobs for the week {} to {}",
        unscheduled.len(),
        start_of_week,
        end_of_week
    );

    for opportunity in unscheduled {
        let member = &opportunity["member"];
        let member_name =
            json::attribute_from_value(&member, "name").expect("Unable to grab client name");
        println!("{}", member_name);
    }

    Ok(())
}
