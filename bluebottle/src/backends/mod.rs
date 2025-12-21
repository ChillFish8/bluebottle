use std::fmt::{Display, Formatter};
use std::str::FromStr;

use rusqlite::ToSql;
use rusqlite::types::{FromSql, FromSqlError};
use serde_json::Value;

mod http;
pub mod jellyfin;

/// The backend provides access to media information required by the Bluebottle UI.
pub trait Backend {}

/// The backend trait for initialising the backend from a persisted state.
pub trait BackendInit: Sized {
    /// Load the backend from some persisted context state.
    fn from_context(context: Value) -> Result<Self, snafu::Whatever>;
}

/// A unique identifier assigned to the backend.
pub type BackendId = uuid::Uuid;

#[derive(Clone)]
/// Information that describes a media library backend and how to re-create it.
pub struct BackendInitState {
    /// The unique identifier of the backend.
    pub id: uuid::Uuid,
    /// The type of the backend.
    pub kind: BackendKind,
    /// Initialisation context.
    pub context: Value,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
/// The backend type encompassing all supported backends.
pub enum BackendKind {
    Jellyfin,
}

impl BackendKind {
    /// Returns the backend kind as a static string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Jellyfin => "jellyfin",
        }
    }
}

impl Display for BackendKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BackendKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "jellyfin" => Ok(Self::Jellyfin),
            _ => Err(format!("unknown backend kind: {s}")),
        }
    }
}

impl FromSql for BackendKind {
    fn column_result(
        value: rusqlite::types::ValueRef<'_>,
    ) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        Self::from_str(s).map_err(|e| FromSqlError::Other(e.into()))
    }
}

impl ToSql for BackendKind {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let bytes = self.as_str().as_bytes();
        Ok(rusqlite::types::ToSqlOutput::Borrowed(
            rusqlite::types::ValueRef::Text(bytes),
        ))
    }
}
