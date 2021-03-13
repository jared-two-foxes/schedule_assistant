//@todo: Create a trait to define the endpoint?
use reqwest::blocking::RequestBuilder;


pub trait Authentication {
    fn apply(&self, request_builder: RequestBuilder) -> RequestBuilder;
}

pub struct BasicAuthentication {
    pub username: String,
    pub password: String,
}

impl Authentication for BasicAuthentication {
    fn apply(&self, request_builder: RequestBuilder) -> RequestBuilder {
        request_builder.basic_auth(self.username.clone(), Some(self.password.clone()))
    }
}

pub struct SubdomainAuthentication {
    pub subdomain: String,
    pub password: String,
}

impl Authentication for SubdomainAuthentication {
    fn apply(&self, request_builder: RequestBuilder) -> RequestBuilder {
        request_builder
            .header("X-SUBDOMAIN", self.subdomain.clone())
            .header("X-AUTH-TOKEN", self.password.clone())        
    }
}
