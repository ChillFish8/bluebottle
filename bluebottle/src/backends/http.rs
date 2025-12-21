use std::sync::LazyLock;

use reqwest::header::HeaderValue;
use reqwest::{Method, header};

static ACCEPT_INVALID_CERTS: LazyLock<bool> =
    LazyLock::new(|| std::env::var("BLUEBOTTLE_ACCEPT_INVALID_CERTS").is_ok());
static ACCEPT_INVALID_HOSTNAME: LazyLock<bool> =
    LazyLock::new(|| std::env::var("BLUEBOTTLE_ACCEPT_INVALID_HOSTNAME").is_ok());

/// The core HTTP client for all backends.
pub struct HttpClient {
    client: reqwest::Client,
    base_url: url::Url,
}

impl HttpClient {
    /// Creates a new standard [HttpClient] with the provided base URL.
    pub fn new(base_url: url::Url) -> Self {
        tracing::info!(
            base_url = %base_url,
            accept_invalid_certs = *ACCEPT_INVALID_CERTS,
            accept_invalid_hostname = *ACCEPT_INVALID_HOSTNAME,
            "creating new http client",
        );

        let client = create_default_builder()
            .build()
            .expect("create new http client");

        Self { client, base_url }
    }

    fn set_default_headers(&mut self, headers: header::HeaderMap) {
        self.client = create_default_builder()
            .default_headers(headers)
            .build()
            .expect("create new http client");
    }

    /// Add basic authentication to the client to inject on every request.
    pub fn add_basic_auth(&mut self, username: &str, password: Option<&str>) {
        let mut map = header::HeaderMap::new();
        map.insert(header::AUTHORIZATION, basic_auth(username, password));
        self.set_default_headers(map);
    }

    /// Add a bearer based authentication token.
    pub fn add_bearer_auth(&mut self, access_token: &str) {
        let bearer = format!("Bearer {access_token}");
        let value = HeaderValue::from_str(&bearer)
            .expect("bearer token should be valid header value");
        let mut map = header::HeaderMap::new();
        map.insert(header::AUTHORIZATION, value);
        self.set_default_headers(map);
    }

    /// Add token based authentication with no `Bearer` prefix.
    pub fn add_token_auth(&mut self, access_token: &str) {
        let value = HeaderValue::from_str(access_token)
            .expect("bearer token should be valid header value");
        let mut map = header::HeaderMap::new();
        map.insert(header::AUTHORIZATION, value);
        self.set_default_headers(map);
    }

    /// Create a new GET request.
    pub fn get(&self, endpoint: &str) -> reqwest::RequestBuilder {
        self.request(Method::GET, endpoint)
    }

    /// Create a new POST request.
    pub fn post(&self, endpoint: &str) -> reqwest::RequestBuilder {
        self.request(Method::PUT, endpoint)
    }

    /// Create a new PUT request.
    pub fn put(&self, endpoint: &str) -> reqwest::RequestBuilder {
        self.request(Method::PUT, endpoint)
    }

    /// CREATE a new DELETE request.
    pub fn delete(&self, endpoint: &str) -> reqwest::RequestBuilder {
        self.request(Method::DELETE, endpoint)
    }

    fn request(&self, method: Method, endpoint: &str) -> reqwest::RequestBuilder {
        let url = self.base_url.join(endpoint).expect("join endpoint to base");
        self.client.request(method, url)
    }
}

fn create_default_builder() -> reqwest::ClientBuilder {
    reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(*ACCEPT_INVALID_CERTS)
        .danger_accept_invalid_hostnames(*ACCEPT_INVALID_HOSTNAME)
}

fn basic_auth<U, P>(username: U, password: Option<P>) -> HeaderValue
where
    U: std::fmt::Display,
    P: std::fmt::Display,
{
    use std::io::Write;

    use base64::prelude::BASE64_STANDARD;
    use base64::write::EncoderWriter;

    let mut buf = b"Basic ".to_vec();
    {
        let mut encoder = EncoderWriter::new(&mut buf, &BASE64_STANDARD);
        let _ = write!(encoder, "{username}:");
        if let Some(password) = password {
            let _ = write!(encoder, "{password}");
        }
    }
    let mut header =
        HeaderValue::from_bytes(&buf).expect("base64 is always valid HeaderValue");
    header.set_sensitive(true);
    header
}
