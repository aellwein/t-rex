//! Error bodies: the JSON the server sends with every failed request.
//!
//! # Examples
//!
//! ```
//! use dto::ErrorBody;
//!
//! let body: ErrorBody = serde_json::from_str(r#"{"error":"name must not be empty"}"#).unwrap();
//! assert_eq!(body.error, "name must not be empty");
//! ```

use serde::{Deserialize, Serialize};

/// JSON body returned with every non-success status code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    /// Human-readable reason, in English. Shown to the user as it is.
    pub error: String,
}
