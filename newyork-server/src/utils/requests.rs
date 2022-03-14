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
    client.c.get(client.base_url.to_owned() + path)
}


pub fn post(client: &HttpClient, path: &str) -> RequestBuilder {
    client.c.post(client.base_url.to_owned() + path)
}
