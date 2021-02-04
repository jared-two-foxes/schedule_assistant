//
//
// TODO:
// 1. Need to handle pre-specified client required delivery times.
// 2. Need to store the any timing windows required by the client somewhere, somehow.  This needs to be independant of servicem8 and
//      if a window of an extended period the algorithm only needs to ensure that the found path falls somewhere in that time.

use chrono::{DateTime, Duration, Local};
use std::cmp;

mod comms;
mod geolocate;
mod retrieve;

static HOME_ADDRESS: &str = "44b Henderson Valley Road, Henderson, Auckland";
static HOME_PT: (f64, f64) = (174.62852, -36.886249);

fn extract_address(o: &serde_json::Value) -> String {
    let destination = &o["destination"];
    let address = &destination["address"];
    let street = address["street"].as_str().unwrap_or("");
    let city = address["city"].as_str().unwrap_or("");
    let county = address["county"].as_str().unwrap_or("");
    let postcode = address["postcode"].as_str().unwrap_or("");

    let out = format!("{},{},{},{}", street, city, county, postcode);

    out
}

#[derive(Clone, Debug, PartialEq)]
enum JobType {
    Packing,
    Delivery,
    Collection,
}

#[derive(Clone, Debug)]
struct Job {
    address: String,
    location: (f64, f64), // Lat/Long of destination
    job_type: JobType,    // Enum for the job role.
    reserve: Duration,    // amount of time to reserve for the booking?
}

struct SearchNode {
    edge: (usize, usize),
    value: Duration,
}

fn create_job(opportunity: &serde_json::Value, job_type: JobType) -> Option<Job> {
    let address = extract_address(&opportunity);
    let location = geolocate::locate(&address)?;

    Some(Job {
        address,
        location,
        job_type,
        reserve: Duration::minutes(30),
    })
}


// Inject the required warehouse stops for this route?
//   1. We go to the warehouse between any collection and delivery.
fn validate_route(route: &[usize], jobs: &[Job]) -> Vec<usize> {
    let mut validated_route = Vec::<usize>::new();
    for i in 0..route.len() {
        let insert = match validated_route.last() {
            Some(j) => {
                let w = *j;
                let v = jobs[w].job_type == JobType::Collection;
                let y = jobs[i].job_type != JobType::Collection;
                v && y
            }
            None => true,
        };

        if insert {
            validated_route.push(jobs.len() - 1); //< Placeholder for home...
        }
        validated_route.push(i);
    }

    // And finally return to the warehouse at the end of the route.
    validated_route.push(jobs.len() - 1);
    validated_route
}

fn calculate_route_distance(route: &[usize], edges: &[SearchNode], jobs: &[Job]) -> Duration {
    let mut r = Duration::seconds(0);
    for i in 0..(route.len() - 1) {
        for e in edges {
            if e.edge.0 == cmp::min(route[i], route[i + 1])
                && e.edge.1 == cmp::max(route[i], route[i + 1])
            {
                r = r + e.value + jobs[route[i]].reserve;
            }
        }
    }

    // And finally add the elapsed time of the final job
    if route.len() > 1 {
        let i = route[route.len() - 1];
        if jobs[route[i - 1]].job_type == JobType::Collection {
            // Add unpacking time. (30min..?)
            r = r + jobs[i].reserve;
        }
    }
    r
}

fn main() {
    dotenv::dotenv().expect("Failed to read .env file");

    let today = Local::today();
    let date = today + Duration::days(3);

    // Pull all currentrms::opportunities.
    let opportunities = retrieve::opportunities();

    // Create jobs for the given day.
    let mut jobs = Vec::new();
    for o in &opportunities {
        // State == 3 is an order.
        if o["state"].as_i64().unwrap_or(0) != 3 {
            continue;
        }

        // Create a job for the delivery if it set for the given day.
        let starts_at_utc = DateTime::parse_from_rfc3339(o["starts_at"].as_str().unwrap()).unwrap();
        let starts_at_local: DateTime<Local> = DateTime::from(starts_at_utc);
        if starts_at_local.date() == date {
            match create_job(&o, JobType::Delivery) {
                Some(job) => jobs.push(job),
                None => println!("Unable to find location for job"),
            }
        }

        // Create a job for the collection if it set for on the given day.
        let ends_at_utc = DateTime::parse_from_rfc3339(o["ends_at"].as_str().unwrap()).unwrap();
        let ends_at_local: DateTime<Local> = DateTime::from(ends_at_utc);
        if ends_at_local.date() == date {
            match create_job(&o, JobType::Collection) {
                Some(job) => jobs.push(job),
                None => println!("Unable to find location for job"),
            }
        }
    }

    println!("There are {} jobs with scheduled on {}", jobs.len(), date);

    for j in &jobs {
        println!("{:?}", j);
    }

    // Push the warehouse at the end of the list.
    jobs.push(Job {
        address: String::from(HOME_ADDRESS),
        location: HOME_PT,
        job_type: JobType::Packing,
        reserve: Duration::minutes(0),
    });

    // Create an enumeration of pairs in this vector, we need one for every possible combination.
    let mut combinations = Vec::new();
    let count = jobs.iter().count();
    for i in 0..count {
        for j in i + 1..count {
            let edge = (i, j);

            // We also need the distance between each of these sets of waypoints
            let coords = [jobs[i].location, jobs[j].location];
            let value = match geolocate::directions(&coords) {
                Ok(json) => {
                    // Parse the returned json string to extrat the distance and time values.
                    assert!( json["code"].as_str().unwrap() == "Okay" );
                    let routes = &json["routes"];

                    //@note:  Do we care about any other routes other than the first?
                    match routes[0]["duration"].as_f64() {
                        Some(duration) => duration,
                        None => {
                            println!("Unable to find duration for edge?");
                            0.0
                        }
                    }
                }
                Err(err) => {
                    println!(
                        "Unable to find a route between {} & {}\n{}",
                        edge.0, edge.1, err
                    );
                    0.0
                }
            };

            combinations.push(SearchNode {
                edge,
                value: Duration::seconds(value as i64),
            });
        }
    }

    // Find the shortest path that travels to all of the nodes with the smallest time taken as possible
    let length = jobs.len() - 1; //< because we dont want to include the home point in this.
    let mut pts = Vec::with_capacity(length);
    for i in 0..length {
        pts.push(i);
    }
    let mut c = vec![0; length];
    let mut i = 0;
    println!("{:?}", pts);
    let mut route = validate_route(&pts, &jobs);
    let mut minimum_distance = calculate_route_distance(&route, &combinations, &jobs);

    while i < length {
        if c[i] < i {
            if i % 2 == 0 {
                pts.swap(i, 0);
            } else {
                pts.swap(c[i], i);
            }

            println!("{:?}", pts);

            // This is a permutation, check the travel duration
            let r = validate_route(&pts, &jobs);
            let d = calculate_route_distance(&r, &combinations, &jobs);

            if d < minimum_distance {
                minimum_distance = d;
                route = r;
            }

            c[i] += 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }

    println!(
        "\n{:?}, {}",
        route,
        minimum_distance.num_seconds() as f64 / 60.0
    );
}
