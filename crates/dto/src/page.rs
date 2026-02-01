//! Pagination for list endpoints. A list returns one [`Page`]; the request picks the window with
//! [`PageQuery`]. Without parameters the first [`DEFAULT_LIMIT`] items are returned.

use serde::{Deserialize, Serialize};

/// Items returned when a request gives no `limit`.
pub const DEFAULT_LIMIT: u64 = 50;
/// Largest `limit` a request may ask for. Larger values are rejected, not clamped.
pub const MAX_LIMIT: u64 = 200;

/// Query parameters that choose a window of a list: `?limit=50&offset=0`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageQuery {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

impl PageQuery {
    /// The `(limit, offset)` to use. Fails if `limit` is zero or above [`MAX_LIMIT`].
    pub fn window(&self) -> Result<(u64, u64), String> {
        let limit = self.limit.unwrap_or(DEFAULT_LIMIT);
        if limit == 0 || limit > MAX_LIMIT {
            return Err(format!("limit must be between 1 and {MAX_LIMIT}"));
        }
        Ok((limit, self.offset.unwrap_or(0)))
    }
}

/// One window of a list. `total` counts all items of the list, not only this window, so a client
/// knows whether to ask for the next one: there is more while `offset + items.len() < total`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

impl<T> Page<T> {
    /// True if the list has items after this window.
    pub fn has_more(&self) -> bool {
        self.offset + (self.items.len() as u64) < self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_parameters_means_the_default_window() {
        assert_eq!(PageQuery::default().window(), Ok((DEFAULT_LIMIT, 0)));
    }

    #[test]
    fn limit_must_be_between_one_and_the_maximum() {
        let limit = |limit| {
            PageQuery {
                limit: Some(limit),
                offset: None,
            }
            .window()
        };
        assert!(limit(0).is_err());
        assert!(limit(1).is_ok());
        assert!(limit(MAX_LIMIT).is_ok());
        assert!(limit(MAX_LIMIT + 1).is_err());
    }

    #[test]
    fn more_items_follow_while_the_window_ends_before_the_total() {
        let page = |offset, count, total| Page {
            items: vec![0; count],
            total,
            limit: 2,
            offset,
        };
        assert!(page(0, 2, 3).has_more());
        assert!(!page(2, 1, 3).has_more());
        assert!(!page(0, 0, 0).has_more());
    }
}
