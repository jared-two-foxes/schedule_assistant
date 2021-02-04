// This module should contain all the functions related to the querying of locations,
// processing of directions etc.

use std::env;

use crate::comms;

static MAPBOX_URL: &str = "http://api.mapbox.com";

fn geocoding(location: &str) -> reqwest::Result<serde_json::Value> {
    let url = format!(
        "{}/geocoding/v5/{}/{}.json?access_token={}",
        MAPBOX_URL,
        "mapbox.places",
        location,
        env::var("MAPBOX_ACCESS_TOKEN").expect("MAPBOX_ACCESS_TOKEN not found")
    );
    comms::get(&url)
}

pub fn locate(location: &str) -> Option<(f64, f64)> {
    // Attempt to find the locaiton supplied.
    let json = geocoding(&location).ok()?;

    // Features is an array! Assume the first element is the one we want.
    let center = &json["features"][0]["center"];
    Some((center[0].as_f64()?, center[1].as_f64()?))
}

pub fn directions(coordinates: &[(f64, f64)]) -> reqwest::Result<serde_json::Value> {
    //@todo: Need to url-encode the coordinates?
    let coords = coordinates
        .iter()
        .map(|x| format!("{},{}", x.0, x.1))
        .collect::<Vec<_>>()
        .join(";");
    let url = format!(
        "{}/directions/v5/{}/{}/{}?alternatives=false&steps=false&access_token={}",
        MAPBOX_URL,
        "mapbox",
        "driving",
        coords,
        env::var("MAPBOX_ACCESS_TOKEN").expect("MAPBOX_ACCESS_TOKEN not found")
    );
    comms::get(&url)
}
