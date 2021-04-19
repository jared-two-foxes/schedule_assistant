// ---------------------------------------------------------------------
// name: Check
// type: Command line applicaiton.
// desc: Lists any current-rms opportunities which havent been
//       scheduled as jobs in servicem8
// ---------------------------------------------------------------------

// Tasks/Todo/Ideas:
// -----------------
// [] Pull the clients email out of servicem8
// [] If there is no email address dont bother attempting to send.
// [] Add stmp server & login to the env file.
// [] Need to restrict the opportunities to those that are confirmed.
// [] gather_opportunities parameters should be optional, if not present
//    the either all the opportunities should be gathered, all since
//    the passed start date, or those for the specified duration where
//    the start date is today

use anyhow;
use chrono::prelude::*;
use chrono::Duration;
use schedule_assistant::authentication::AuthenticationCache;
use schedule_assistant::{current_rms, servicem8};
use serde::Deserialize;

// Data Structures
// ---------------

#[derive(Clone, Deserialize)]
pub struct MemberEmail {
    address: String, 
    type_id: u32,
    email_type_name: String, 
    id: u32
}

#[derive(Clone, Deserialize)]
pub struct Member {
    uuid: u32,
    name: String,
    active: bool,
    emails: Vec<MemberEmail>,
}

#[derive(Clone, Deserialize)]
pub struct Opportunity {
    id: u32,
    member_id: u32, //<?? Interestingly if we have the member_id's etc at this level, do we need the member structs as part of this structure?
    member: Member,
    subject: String,
    starts_at: DateTime<Utc>, //< We need to do some fancy shit for this to serialize properly
    end_at: DateTime<Utc>,
    state: u32,
    state_name: String,
    status: u32,
    status_name: String,
}

#[derive(Deserialize)]
pub struct Job {
    uuid: String,
}



// Functions
// ----------

// main function
//
// lists any confirmed current_rms opportunities that havent been
// scheduled as jobs in servicem8
//
// @tasks:
// [x] calculate start & length from command line arguments
// [] pull current_rms::opportunity's from current_rms endpoint
// [] filter opportunities by optional date parameters
// [] pull servicem8::job's from servicem8 endpoint
// [] iterate current_rms::opportunity's and find matching servicem8::job
// [] list any opportunities that doesnt have a matching job
pub fn main() -> anyhow::Result<()> {
    dotenv::dotenv().expect("Failed to read .env file");

    let args: Vec<String> = std::env::args().collect();
    let start = if args.len() > 1 {
        Utc.datetime_from_str(&args[1], "%Y-%m-%d %H:%M:%S")?.date() //"2021-03-22 00:00:00"
    } else {
        Utc::now().date()
    };
    let length = if args.len() > 2 {
        args[2].parse::<i64>()?
    } else {
        7
    };
    let duration = Duration::days(length);
    let (start_date, end_date) = calculate_window(start, duration);
    let auth_cache = AuthenticationCache::new();
    let opportunities = gather_opportunities(&auth_cache, start_date, end_date);
    let jobs = gather_jobs(&auth_cache);
    let unscheduled_jobs = check_scheduled(&opportunities, &jobs);
    print_opportunities(&unscheduled_jobs);
    Ok(())
}

// calculate_window
//
// converts a start and duration to a start and end date
fn calculate_window(start: Date<Utc>, duration: Duration) -> (Date<Utc>, Date<Utc>) {
    let end = start + duration;
    (start, end)
}

// gather_opportunities
//
// Retrieves the current_rms::opportunity structures that fall
// within a given time period.
//
// @tasks:
// [x] establish connection to the opportunity endpoint,
// [] setup authorization,
// [x] retrieve all of the opportunities
// [x] filter for those that fit in the specified window
//
// note:
// - Can we restrict the retrieval to the specified information or
//   do we need to do that post retrieval?
fn gather_opportunities(
    auth_cache: &AuthenticationCache,
    start_date: Date<Utc>,
    end_date: Date<Utc>,
) -> Vec<Opportunity> {
    let opportunities = current_rms::opportunities(auth_cache).unwrap();
    opportunities
        .into_iter()
        .filter(current_rms::opportunity_is_confirmed)
        .filter(|opportunity| {
            schedule_assistant::opportunity_within_date_range(&opportunity, &start_date, &end_date)
                .unwrap_or(false)
        })
        .map(|opportunity| serde_json::from_value(opportunity).unwrap())
        .collect::<Vec<Opportunity>>()
}

// gather_jobs
//
// Retrieves all of the servicem8::job elements
//
// tasks:
// [x] establish connection to the servicem8 jobs endpoint,
// [x] setup authorization
// [x] retrieve all of the jobs
fn gather_jobs(auth_cache: &AuthenticationCache) -> Vec<Job> {
    servicem8::jobs(auth_cache)
        .unwrap()
        .into_iter()
        .map(|value| serde_json::from_value(value).unwrap())
        .collect()
}

// check_scheduled(opportunities_to_check, jobs)
//
// iterates over the opportunities_to_check, compares against jobs, outputs
// a list of unscheduled opportunities.
fn check_scheduled(opportunities_to_check: &Vec<Opportunity>, jobs: &Vec<Job>) -> Vec<Opportunity> {
    opportunities_to_check.iter().cloned().collect()
}

// print_opportunities
//
// Displays details of a collection of current_rms::Opportunity items
//
// @tasks:
// [] Determine what details to show?
// [] Show details of given opportunities to console.
fn print_opportunities(opportunities: &Vec<Opportunity>) {
    for opportunity in opportunities {
        println!("{} ({})", opportunity.member.name, opportunity.id);
    }
}

// fn main() -> anyhow::Result<()> {
//     dotenv::dotenv().expect("Failed to read .env file");

//     let auth_cache = schedule_assistant::authentication::AuthenticationCache::new();
//     let args: Vec<String> = env::args().collect();
//     let start = if args.len() > 1 {
//         Utc.datetime_from_str(&args[1], "%Y-%m-%d %H:%M:%S")? //"2021-03-22 00:00:00"
//     } else {
//         Utc::now()
//     };
//     let (start_of_week, end_of_week) = calculate_date_range(start); //< Calculate the date filter; Next week, starting from the following monday.
//     let unscheduled = schedule_assistant::check_bookings(&auth_cache, start_of_week, end_of_week)?;

//     println!(
//         "There are {} unscheduled opportunities for the week {} to {}",
//         unscheduled.len(),
//         start_of_week,
//         end_of_week
//     );

//     for opportunity in unscheduled {
//         let member = &opportunity["member"];
//         let member_name = json::attribute_from_value(&member, "name")
//             .expect("Unable to extract client name from opportunity");
//         let opportunity_id = opportunity["id"].as_u64().unwrap();
//         println!("{} ({})", member_name, opportunity_id);
//     }

//     Ok(())
// }
