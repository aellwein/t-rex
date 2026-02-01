use serde::{Deserialize, Serialize};

/// JSON body returned with every non-success status code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: String,
}
