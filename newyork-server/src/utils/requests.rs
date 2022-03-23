use reqwest::blocking::RequestBuilder;

pub struct HttpClient {
    c: reqwest::blocking::Client,
    base_url: String,
}

impl HttpClient {
    pub fn new(base_url: String) -> HttpClient {
        HttpClient {
            c: reqwest::blocking::Client::new(),
            base_url,
        }
    }
}

pub fn get(client: &HttpClient, path: &str) -> RequestBuilder {
    client.c.get(format!("{}{}", client.base_url, path))
}

pub fn post(client: &HttpClient, path: &str) -> RequestBuilder {
    client.c.post(format!("{}{}", client.base_url, path))
}
