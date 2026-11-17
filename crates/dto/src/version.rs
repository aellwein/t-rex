//! Version information: the build the server runs, shown in the page footer.

use serde::{Deserialize, Serialize};

/// The build the server runs, for the footer. Returned by `GET /api/v1/version`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Git description of the build, e.g. the commit hash.
    pub version: String,
}
