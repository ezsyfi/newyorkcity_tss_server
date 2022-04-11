use reqwest::RequestBuilder;

pub struct HttpClient {
    c: reqwest::Client,
    base_url: String,
}

impl HttpClient {
    pub fn new(base_url: String) -> HttpClient {
        HttpClient {
            c: reqwest::Client::new(),
            base_url,
        }
    }
}

pub async fn get(client: &HttpClient, path: &str) -> RequestBuilder {
    client.c.get(format!("{}{}", client.base_url, path))
}

pub async fn post(client: &HttpClient, path: &str) -> RequestBuilder {
    client.c.post(format!("{}{}", client.base_url, path))
}
