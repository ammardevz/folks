//! Pagination types for list endpoints built on `UserQueryRequest` & co.

use serde::{Deserialize, Serialize};

/// Largest page size [`PageRequest`] will hand out.
pub const MAX_PER_PAGE: u64 = 100;
/// Page size used when none is given.
pub const DEFAULT_PER_PAGE: u64 = 20;

/// A 1-based page request. Out-of-range values are clamped, not rejected, so
/// `offset()`/`limit()` are always safe to pass to a database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRequest {
    #[serde(default = "first_page")]
    pub page: u64,
    #[serde(default = "default_per_page")]
    pub per_page: u64,
}

fn first_page() -> u64 {
    1
}

fn default_per_page() -> u64 {
    DEFAULT_PER_PAGE
}

impl Default for PageRequest {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: DEFAULT_PER_PAGE,
        }
    }
}

impl PageRequest {
    pub fn new(page: u64, per_page: u64) -> Self {
        Self { page, per_page }
    }

    /// `per_page` clamped to `1..=MAX_PER_PAGE`.
    pub fn limit(&self) -> u64 {
        self.per_page.clamp(1, MAX_PER_PAGE)
    }

    /// Rows to skip; page numbers below 1 count as page 1.
    pub fn offset(&self) -> u64 {
        (self.page.max(1) - 1).saturating_mul(self.limit())
    }
}

/// One page of results plus the total number of matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: u64, request: &PageRequest) -> Self {
        Self {
            items,
            total,
            page: request.page.max(1),
            per_page: request.limit(),
        }
    }

    pub fn total_pages(&self) -> u64 {
        self.total.div_ceil(self.per_page.max(1))
    }

    /// Converts the items, keeping the paging data:
    /// `page.map(UserResponse::from)`.
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            total: self.total,
            page: self.page,
            per_page: self.per_page,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_and_limits_are_clamped() {
        assert_eq!(PageRequest::default().offset(), 0);
        assert_eq!(PageRequest::new(3, 10).offset(), 20);
        assert_eq!(PageRequest::new(0, 10).offset(), 0);
        assert_eq!(PageRequest::new(1, 0).limit(), 1);
        assert_eq!(PageRequest::new(1, 10_000).limit(), MAX_PER_PAGE);
        // Huge page numbers saturate instead of overflowing.
        assert_eq!(PageRequest::new(u64::MAX, 100).offset(), u64::MAX);
    }

    #[test]
    fn pages_count_and_map() {
        let req = PageRequest::new(2, 10);
        let page = Page::new(vec![1, 2, 3], 25, &req);
        assert_eq!((page.page, page.per_page, page.total_pages()), (2, 10, 3));
        let mapped = page.map(|n| n.to_string());
        assert_eq!(mapped.items, ["1", "2", "3"]);
        assert_eq!(mapped.total, 25);
    }

    #[test]
    fn request_defaults_when_omitted() {
        let r: PageRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(r, PageRequest::default());
    }
}
