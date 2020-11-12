use reqwest::blocking::Client;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

extern crate dotenv;

fn main() {
    dotenv::dotenv().expect("Failed to read .env file");

    // Convert the input file into geolocation coordinates
    let mut vec = Vec::new();
    if let Ok(lines) = read_lines("./places") {
        for line in lines {
            if let Ok(location) = line {
                match geolocate(&location) {
                    Ok(json) => {
                        // Features is an array! Assume the first element is the one we want.
                        let center = &json["features"][0]["center"];

                        // Grab the coordinates and dump them into a Vec.
                        vec.push(format!(
                            "{}, {:?}",
                            location,
                            (center[0].as_f64().unwrap(), center[1].as_f64().unwrap())
                        ));
                    }
                    _ => {
                        vec.push(format!("{}, unknown", location));
                    }
                }
            }
        }
    }

    // Dump the contents of the file to the console.
    println!("{:?}", vec);
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file
fn read_lines<P>(filename: P) -> io::Result<io::Lines<BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn geolocate(location: &str) -> reqwest::Result<serde_json::Value> {
    //@todo: Need to url-encde the location.
    let url = format!(
        "{}/geocoding/v5/{}/{}.json?access_token={}",
        "http://api.mapbox.com",
        "mapbox.places",
        location,
        env::var("MAPBOX_ACCESS_TOKEN").expect("MAPBOX_ACCESS_TOKEN not found")
    );
    let client = Client::new();
    let response = client.get(&url).send()?;

    response.json()
}
