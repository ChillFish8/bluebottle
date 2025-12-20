use serde_json::Value;
use snafu::ResultExt;

use crate::backends::http::HttpClient;
use crate::backends::{Backend, BackendInit};

mod auth;

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
/// The context for the Jellyfin backend.
struct Context {
    server_url: url::Url,
    access_token: String,
}

/// A backend client for the Jellkfin media library.
pub struct Jellyfin {
    client: HttpClient,
}

impl BackendInit for Jellyfin {
    fn from_context(context: Value) -> Result<Self, snafu::Whatever> {
        let context: Context = serde_json::from_value(context)
            .whatever_context("deserialize persisted backend context")?;

        let mut client = HttpClient::new(context.server_url);
        client.add_token_auth(&context.access_token);

        Ok(Jellyfin { client })
    }
}

impl Backend for Jellyfin {}
