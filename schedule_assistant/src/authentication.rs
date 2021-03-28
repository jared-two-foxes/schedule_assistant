use crate::retrieve::authentication::*;
use std::env;

pub struct AuthenticationCache {
    pub currentrms: BearerAuthentication,
    pub servicem8: BasicAuthentication,
}

impl AuthenticationCache {
    pub fn new() -> AuthenticationCache {
        AuthenticationCache {
            currentrms: current_rms().expect("Unable to create the current_rms endpoint"),
            servicem8: servicem8(),
        }
    }

    pub fn servicem8(&self) -> &impl Authentication {
        &self.servicem8
    }

    pub fn currentrms(&self) -> &impl Authentication {
        &self.currentrms
    }
}

fn current_rms() -> reqwest::Result<BearerAuthentication> {
    use oauth2::basic::BasicClient;
    use oauth2::reqwest::http_client;
    use oauth2::{
        AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
        RedirectUrl, TokenResponse, TokenUrl,
    };
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use url::Url;

    let client_id = ClientId::new(
        env::var("CURRENT_CLIENT_ID").expect("Missing the CURRENT_CLIENT_ID environment variable."),
    );
    let client_secret = ClientSecret::new(
        env::var("CURRENT_CLIENT_SECRET")
            .expect("Missing the CURRENT_CLIENT_SECRET environment variable."),
    );
    let subdomain = env::var("CURRENT_DOMAIN_NAME")
        .expect("Missing the CURRENT_DOMAIN_NAME environment variable.");

    let auth_url = AuthUrl::new(format!(
        "https://{}.current-rms.com/oauth2/authorize",
        subdomain
    ))
    .expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new(format!(
        "https://{}.current-rms.com/oauth2/token",
        subdomain
    ))
    .expect("Invalid token endpoint URL");

    // Set up the config for the Github OAuth2 process.
    let client = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
        // This example will be running its own server at localhost:8080.
        // See below for the server implementation.
        .set_redirect_url(
            RedirectUrl::new("http://localhost:8080".to_string()).expect("Invalid redirect URL"),
        );

    // Generate the authorization URL to which we'll redirect the user.
    let (authorize_url, csrf_state) = client.authorize_url(CsrfToken::new_random).url();

    println!(
        "Open this URL in your browser:\n{}\n",
        authorize_url.to_string()
    );

    // A very naive implementation of the redirect server.
    let mut token = String::from("");
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            let code;
            let state;
            {
                let mut reader = BufReader::new(&stream);

                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();

                let redirect_url = request_line.split_whitespace().nth(1).unwrap();
                let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

                let code_pair = url
                    .query_pairs()
                    .find(|pair| {
                        let &(ref key, _) = pair;
                        key == "code"
                    })
                    .unwrap();

                let (_, value) = code_pair;
                code = AuthorizationCode::new(value.into_owned());

                let state_pair = url
                    .query_pairs()
                    .find(|pair| {
                        let &(ref key, _) = pair;
                        key == "state"
                    })
                    .unwrap();

                let (_, value) = state_pair;
                state = CsrfToken::new(value.into_owned());
            }

            let message = "Go back to your terminal :)";
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                message.len(),
                message
            );
            stream.write_all(response.as_bytes()).unwrap();

            println!("Github returned the following code:\n{}\n", code.secret());
            println!(
                "Github returned the following state:\n{} (expected `{}`)\n",
                state.secret(),
                csrf_state.secret()
            );

            // Exchange the code with a token.
            let token_res = client.exchange_code(code).request(http_client).unwrap();
            println!("Github returned the following token:\n{:?}\n", token_res);
            token = token_res.access_token().secret().clone();

            // The server will terminate itself after collecting the first code.
            break;
        }
    }

    return Ok(BearerAuthentication { subdomain, token });
}

fn servicem8() -> BasicAuthentication {
    let username = env::var("SERVICEM8_USERNAME").expect("SERVICEM8_USERNAME not found");
    let password = env::var("SERVICEM8_PASSWORD").expect("SERVICEM8_PASSWORD not found");
    BasicAuthentication { username, password }
}
