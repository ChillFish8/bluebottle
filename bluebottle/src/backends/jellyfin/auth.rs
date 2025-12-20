use reqwest::StatusCode;
use serde_json::json;
use snafu::ResultExt;

use super::Context;
use crate::backends::http::HttpClient;

static USER_AUTHENTICATION_ENDPOINT: &str = "/Users/AuthenticateByName";

/// Creates a new backend [Context] for the Jellyfin media library.
pub async fn create_backend_context(
    url: url::Url,
    username: String,
    password: String,
) -> Result<Context, CreateContextError> {
    let client = HttpClient::new(url.clone());

    let payload = json!({
      "Username": username,
      "Pw": password,
    });

    let resp = client
        .post(USER_AUTHENTICATION_ENDPOINT)
        .json(&payload)
        .send()
        .await
        .context(ConnectionSnafu)?
        .error_for_status()?;

    let payload: AuthenticationBody = resp
        .json()
        .await
        .map_err(|_| CreateContextError::InvalidResponse)?;

    Ok(Context {
        server_url: url,
        access_token: payload.access_token,
    })
}

#[derive(Debug, snafu::Snafu)]
/// An error preventing the system from creating a new [Context] instance.
pub enum CreateContextError {
    #[snafu(display("{}", source))]
    Connection { source: reqwest::Error },
    #[snafu(display(
        "({}) {}: {}",
        status_code.as_u16(),
        status_code.canonical_reason().unwrap_or(""),
        message,
    ))]
    Request {
        /// The status code of the request that failed.
        status_code: StatusCode,
        /// Additional context message from the service.
        message: String,
    },
    #[snafu(display("server returned an invalid response payload"))]
    InvalidResponse,
}

impl From<reqwest::Error> for CreateContextError {
    fn from(source: reqwest::Error) -> Self {
        if let Some(status_code) = source.status() {
            Self::Request {
                status_code,
                message: source.to_string(),
            }
        } else {
            Self::Connection { source }
        }
    }
}

#[derive(serde_derive::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AuthenticationBody {
    access_token: String,
}
