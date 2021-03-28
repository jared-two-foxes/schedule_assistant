//@todo Handle the "Sale" items during Finalise.
//@todo Add the ability to do some basic filtering here, like passing through that we only
//      want active opportunities or something.

pub mod current_rms {
    use crate::retrieve::endpoint::{BasicEndpoint, Endpoint, PagedEndpoint};
    
    static BASE_URL: &str = "https://api.current-rms.com/api/v1";

    pub fn opportunities() -> PagedEndpoint {
        let base_url = format!("{}/{}", BASE_URL, "opportunities");
        PagedEndpoint { base_url, page: 0 }
    }

    pub fn mark_as_dead(id: u64) -> impl Endpoint {
        let base_url = format!("{}/opportunities/{}/mark_as_dead", BASE_URL, id);
        BasicEndpoint { base_url }
    }

    pub fn opportunity_print_document_pdf(subdomain: &str, opportunity_id: u64, document_id: u64) -> impl Endpoint {
        let base_url = format!("https://{}.current-rms.com/opportunities/{}/print_document.pdf?document_id={}", subdomain, opportunity_id, document_id);
        BasicEndpoint { base_url }
    }
}

pub mod servicem8 {
    use crate::retrieve::endpoint::BasicEndpoint;
    static BASE_URL: &str = "https://api.servicem8.com/api_1.0/";
    static CLIENTS_URL: &str = "company.json";
    static JOB_ACTIVITIES_URL: &str = "jobactivity.json";
    static JOB_CONTACTS_URL: &str = "jobcontact.json";
    static JOBS_URL: &str = "job.json";

    pub fn clients() -> BasicEndpoint {
        let base_url = format!("{}{}", BASE_URL, CLIENTS_URL);
        BasicEndpoint { base_url }
    }

    pub fn activities() -> BasicEndpoint {
        let base_url = format!("{}{}", BASE_URL, JOB_ACTIVITIES_URL);
        BasicEndpoint { base_url }
    }

    pub fn contacts() -> BasicEndpoint {
        let base_url = format!("{}{}", BASE_URL, JOB_CONTACTS_URL);
        BasicEndpoint { base_url }
    }

    pub fn jobs() -> BasicEndpoint {
        let base_url = format!("{}{}", BASE_URL, JOBS_URL);
        BasicEndpoint { base_url }
    }
}
