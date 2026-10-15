//! Shared bounds for manually paginated reads. No AWS, I/O or error formatting:
//! callers retain their own deadlines, requests, and partial-result policy.
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StopReason {
    RepeatedToken,
    PageLimit,
    ResultLimit,
}

/// Advance once per successfully received page, after processing its rows.
/// Repeated tokens take precedence over bounds to preserve diagnostics.
/// Callers choose collected-page or streamed-result bounds explicitly.
pub(crate) struct Pagination {
    seen: HashSet<String>,
    pages: usize,
    max_pages: Option<usize>,
    result_limit: Option<usize>,
    empty_token_is_end: bool,
}
impl Pagination {
    /// Collected readers treat empty/missing tokens as completion.
    pub(crate) fn new(max_pages: usize) -> Self {
        Self {
            seen: HashSet::new(),
            pages: 0,
            max_pages: Some(max_pages),
            result_limit: None,
            empty_token_is_end: true,
        }
    }

    /// Related-resource readers stream full pages before checking continuation.
    /// Preserve their protocol: only None ends a stream, empty tokens are opaque,
    /// and a final page is complete even when the result count reaches the cap.
    /// Their outer request timeout remains the time bound for empty pages.
    pub(crate) fn streaming(result_limit: usize) -> Self {
        Self {
            seen: HashSet::new(),
            pages: 0,
            max_pages: None,
            result_limit: Some(result_limit),
            empty_token_is_end: false,
        }
    }

    pub(crate) fn advance(&mut self, next: Option<String>) -> Result<Option<String>, StopReason> {
        self.advance_streamed(next, 0)
    }

    /// `count` is the cumulative number of source rows processed, not the number
    /// of references emitted (a source row may emit none or several).
    pub(crate) fn advance_streamed(
        &mut self,
        next: Option<String>,
        count: usize,
    ) -> Result<Option<String>, StopReason> {
        self.pages = self.pages.saturating_add(1);
        let Some(next) = next.filter(|token| !self.empty_token_is_end || !token.is_empty()) else {
            return Ok(None);
        };
        if !self.seen.insert(next.clone()) {
            return Err(StopReason::RepeatedToken);
        }
        if self.result_limit.is_some_and(|limit| count >= limit) {
            return Err(StopReason::ResultLimit);
        }
        if self.max_pages.is_some_and(|limit| self.pages >= limit) {
            return Err(StopReason::PageLimit);
        }
        Ok(Some(next))
    }
}

/// Cap accumulated rows, reporting whether data was omitted or more pages remain.
/// Exactly `limit` rows on a final page are complete; `limit + 1` is not.
/// Callers may collect one extra row to distinguish those cases cheaply.
pub(crate) fn cap_results<T>(rows: &mut Vec<T>, limit: usize, next: Option<&str>) -> bool {
    let has_more = next.is_some_and(|token| !token.is_empty());
    if rows.len() > limit || (rows.len() == limit && has_more) {
        rows.truncate(limit);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_and_empty_tokens_complete_even_on_the_page_bound() {
        for token in [None, Some(String::new())] {
            assert_eq!(Pagination::new(1).advance(token), Ok(None));
        }
    }

    #[test]
    fn tokens_are_opaque_and_returned_without_trimming_or_decoding() {
        let mut pagination = Pagination::new(10);
        for token in [" ", "a+/=", " a ", "a", "A", "資源"] {
            assert_eq!(
                pagination.advance(Some(token.into())),
                Ok(Some(token.into()))
            );
        }
    }

    #[test]
    fn repeated_tokens_and_longer_cycles_are_detected() {
        for tokens in [
            vec!["a", "a"],
            vec!["a", "b", "a"],
            vec!["a", "b", "c", "b"],
        ] {
            let mut pagination = Pagination::new(100);
            for (i, token) in tokens.iter().enumerate() {
                let result = pagination.advance(Some((*token).into()));
                if i + 1 == tokens.len() {
                    assert_eq!(result, Err(StopReason::RepeatedToken));
                } else {
                    assert_eq!(result, Ok(Some((*token).into())));
                }
            }
        }
    }

    #[test]
    fn bound_only_reports_incomplete_when_another_page_is_needed() {
        let mut pagination = Pagination::new(2);
        assert_eq!(pagination.advance(Some("a".into())), Ok(Some("a".into())));
        assert_eq!(pagination.advance(None), Ok(None));
        let mut pagination = Pagination::new(2);
        assert_eq!(pagination.advance(Some("a".into())), Ok(Some("a".into())));
        assert_eq!(
            pagination.advance(Some("b".into())),
            Err(StopReason::PageLimit)
        );
    }

    #[test]
    fn repeated_token_diagnostic_precedes_page_bound() {
        let mut pagination = Pagination::new(2);
        assert!(pagination.advance(Some("same".into())).is_ok());
        assert_eq!(
            pagination.advance(Some("same".into())),
            Err(StopReason::RepeatedToken)
        );
    }

    #[test]
    fn independent_operations_do_not_share_token_history() {
        for _ in 0..2 {
            assert_eq!(
                Pagination::new(2).advance(Some("a".into())),
                Ok(Some("a".into()))
            );
        }
    }

    #[test]
    fn cap_retains_order_and_distinguishes_complete_from_truncated_results() {
        for (len, token, limited) in [
            (0, None, false),
            (2, Some("next"), false),
            (3, None, false),
            (3, Some(""), false),
            (3, Some("next"), true),
            (4, None, true),
            (4, Some(""), true),
            (4, Some("next"), true),
        ] {
            let mut rows: Vec<_> = (0..len).collect();
            assert_eq!(cap_results(&mut rows, 3, token), limited);
            assert_eq!(rows, (0..len.min(3)).collect::<Vec<_>>());
        }
    }

    #[test]
    fn zero_capacity_never_retains_rows_or_hides_more_pages() {
        let mut rows = vec!["first"];
        assert!(cap_results(&mut rows, 0, None));
        assert!(rows.is_empty());
        assert!(!cap_results(&mut rows, 0, None));
        assert!(cap_results(&mut rows, 0, Some("next")));
    }
}

#[cfg(test)]
mod streaming_tests {
    use super::*;

    #[test]
    fn streamed_results_only_cap_when_another_page_remains() {
        for count in [0, 999, 1000, 1200] {
            assert_eq!(
                Pagination::streaming(1000).advance_streamed(None, count),
                Ok(None)
            );
        }
        assert_eq!(
            Pagination::streaming(1000).advance_streamed(Some("next".into()), 999),
            Ok(Some("next".into()))
        );
        for count in [1000, 1200] {
            assert_eq!(
                Pagination::streaming(1000).advance_streamed(Some("next".into()), count),
                Err(StopReason::ResultLimit)
            );
        }
    }

    #[test]
    fn streamed_repeated_tokens_precede_result_limit() {
        let mut pagination = Pagination::streaming(2);
        assert_eq!(
            pagination.advance_streamed(Some("a".into()), 1),
            Ok(Some("a".into()))
        );
        assert_eq!(
            pagination.advance_streamed(Some("a".into()), 2),
            Err(StopReason::RepeatedToken)
        );
    }

    #[test]
    fn streamed_empty_token_remains_opaque_and_cycles_are_detected() {
        let mut pagination = Pagination::streaming(1000);
        assert_eq!(
            pagination.advance_streamed(Some("".into()), 0),
            Ok(Some("".into()))
        );
        assert_eq!(
            pagination.advance_streamed(Some("a".into()), 0),
            Ok(Some("a".into()))
        );
        assert_eq!(
            pagination.advance_streamed(Some("".into()), 0),
            Err(StopReason::RepeatedToken)
        );
    }

    #[test]
    fn streamed_reader_keeps_its_timeout_instead_of_adding_a_page_limit() {
        let mut pagination = Pagination::streaming(1000);
        for index in 0..1500 {
            let token = index.to_string();
            assert_eq!(
                pagination.advance_streamed(Some(token.clone()), 0),
                Ok(Some(token))
            );
        }
    }
}
