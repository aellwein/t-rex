//! The signed-in user, as the app header shows it.

use serde::{Deserialize, Serialize};

/// The signed-in user, as shown in the app header. Returned by `GET /api/v1/me`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Me {
    /// The account's email address, if the identity provider sent one.
    pub email: Option<String>,
    /// The account's display name, if the identity provider sent one.
    pub name: Option<String>,
}
