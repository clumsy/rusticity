use super::{filter_by_field, filter_by_fields, TextFilter};
use std::cell::Cell;

#[test]
fn arrays_vectors_and_lazy_iterators_share_case_insensitive_semantics() {
    let items = vec![
        ["Zulu".to_string(), "café".to_string()],
        ["Alpha".into(), "QUEUE".into()],
    ];
    for query in ["", "zUL", "CAFÉ", "queue", "absent", " "] {
        let array = filter_by_fields(&items, query, |row| [&row[0], &row[1]]);
        let vector = filter_by_fields(&items, query, |row| vec![row[0].as_str(), row[1].as_str()]);
        let iterator = filter_by_fields(&items, query, |row| row.iter());
        assert_eq!(array, vector);
        assert_eq!(array, iterator);
    }
    assert_eq!(
        filter_by_fields(&items, "CAFÉ", |row| row.iter()),
        vec![&items[0]]
    );
    assert_eq!(
        filter_by_fields(&items, "queue", |row| row.iter()),
        vec![&items[1]]
    );
}

#[test]
fn matching_does_not_trim_tokenize_or_change_unicode_casefolding() {
    let query = TextFilter::new(" É ");
    assert!(query.matches("x é y"));
    assert!(!query.matches("é"));
    assert!(TextFilter::new("界").matches("世界"));
    assert!(TextFilter::new("İ").matches("i\u{307}"));
    assert!(!TextFilter::new("ss").matches("ß"));
    assert!(!TextFilter::new("é").matches("e\u{301}"));
}

#[test]
fn empty_query_preserves_all_rows_without_evaluating_accessors() {
    let items = ["last", "first", "last"];
    assert_eq!(
        filter_by_field(&items, "", |_| panic!("must not read fields")),
        items.iter().collect::<Vec<_>>()
    );
    assert_eq!(
        filter_by_fields(&items, "", |_| -> [&str; 0] {
            panic!("must not read fields")
        }),
        items.iter().collect::<Vec<_>>()
    );
    assert!(TextFilter::new("").matches_any(std::iter::empty::<&str>()));
    assert!(!TextFilter::new("x").matches_any(std::iter::empty::<&str>()));
}

#[test]
fn results_retain_borrowed_identity_duplicates_and_source_order() {
    let items = [
        "second".to_string(),
        "first".to_string(),
        "second".to_string(),
    ];
    let rows = filter_by_field(&items, "SECOND", String::as_str);
    assert_eq!(rows.len(), 2);
    assert!(std::ptr::eq(rows[0], &items[0]));
    assert!(std::ptr::eq(rows[1], &items[2]));
    assert_eq!(filter_by_fields(&items, "SECOND", |row| [row]), rows);
}

#[test]
fn iterator_matching_stops_at_first_matching_field() {
    let visited = Cell::new(0);
    let fields = ["match", "unused"]
        .into_iter()
        .inspect(|_| visited.set(visited.get() + 1));
    assert!(TextFilter::new("match").matches_any(fields));
    assert_eq!(visited.get(), 1);
}

#[test]
fn empty_input_and_empty_dynamic_fields_are_handled_consistently() {
    assert!(filter_by_fields(&Vec::<Vec<String>>::new(), "x", |row| row.iter()).is_empty());
    let items = vec![vec![], vec!["x".to_string()]];
    assert_eq!(filter_by_fields(&items, "", |row| row.iter()).len(), 2);
    assert_eq!(
        filter_by_fields(&items, "X", |row| row.iter()),
        vec![&items[1]]
    );
}
