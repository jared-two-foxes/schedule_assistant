
use crate::retrieve::authentication::{
    Authentication, BasicAuthentication, SubdomainAuthentication,
};
use std::env;

pub fn current_rms() -> impl Authentication {
    let subdomain = env::var("CURRENT_DOMAIN_NAME").expect("CURRENT_DOMAIN_NAME not found");
    let password = env::var("CURRENT_ACCESS_TOKEN").expect("CURRENT_ACCESS_TOKEN not found");
    SubdomainAuthentication {
        subdomain,
        password,
    }
}

pub fn servicem8() -> impl Authentication {
    let username = env::var("SERVICEM8_USERNAME").expect("SERVICEM8_USERNAME not found");
    let password = env::var("SERVICEM8_PASSWORD").expect("SERVICEM8_PASSWORD not found");
    BasicAuthentication { username, password }
}
