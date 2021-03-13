//@todo: Create a trait to define the endpoint?

use url::Url;

pub trait Endpoint {
    fn url(&self) -> String;
}

pub struct BasicEndpoint {
    pub base_url: String,
}

impl Endpoint for BasicEndpoint {
    fn url(&self) -> String {
        return self.base_url.clone();
    }
}

pub struct PagedEndpoint {
    pub base_url: String,
    pub page: u32,
}

impl PagedEndpoint {
    pub fn advance(&mut self) {
        self.page += 1;
    }
}

impl Endpoint for PagedEndpoint {
    fn url(&self) -> String {
        let per_page = "100";
        
        let params = vec![
            ("page".to_string(), self.page.to_string()),
            ("per_page".to_string(), per_page.to_string()),
        ];
    
        match Url::parse_with_params(&self.base_url, params) {
            Ok(url) => url.as_str().to_string(),
            // Not quite sure what to do if this call fails.  Returning the base_url 
            // which should allow us to attempt to make the call without parameters,
            // obviously this means that we wouldnt be paging but the option is 
            // returning an Error or something.
            Err(_e) => self.base_url.clone()  
        }
    }
}