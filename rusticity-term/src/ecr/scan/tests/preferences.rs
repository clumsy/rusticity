use super::*;
use crate::{
    common::PageSize,
    preferences::{bindings, PreferenceRow},
};

const TABLE_TABS: [DetailTab; 4] = [
    DetailTab::Scanning,
    DetailTab::Referrers,
    DetailTab::Replication,
    DetailTab::Signing,
];
fn model(app: &App) -> crate::preferences::Preferences {
    bindings::model_for(app, Service::EcrRepositories)
}
fn open_preferences(app: &mut App) {
    press(app, KeyCode::Char('p'), KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::ColumnSelector);
}
fn toggle(app: &mut App, index: usize) {
    app.column_selector_index = index;
    press(app, KeyCode::Char(' '), KeyModifiers::NONE);
}
fn close_preferences(app: &mut App) {
    press(app, KeyCode::Esc, KeyModifiers::NONE);
}

#[test]
fn every_table_tab_uses_its_rendered_schema_and_shared_preferences_actions() {
    for tab_kind in TABLE_TABS {
        let mut app = loaded_table(tab_kind);
        let parent_columns = app.ecr_image_visible_column_ids.clone();
        let parent_page_size = app.ecr_state.images.page_size;
        open_preferences(&mut app);
        let prefs = model(&app);
        let names: Vec<_> = prefs
            .columns()
            .map(|column| column.label.as_str())
            .collect();
        let expected: Vec<_> = tab_kind.columns().iter().map(|(name, _)| *name).collect();
        assert_eq!(names, expected);
        // Standard section navigation and row bounds, not per-tab index arithmetic.
        press(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.column_selector_index, prefs.move_section(0, true));
        press(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(app.column_selector_index, 0);
        let last = *expected.last().unwrap();
        let index = prefs
            .rows()
            .iter()
            .position(|row| matches!(row, PreferenceRow::Column(c) if c.id == last))
            .unwrap();
        toggle(&mut app, index);
        assert!(
            !model(&app)
                .columns()
                .find(|c| c.id == last)
                .unwrap()
                .visible
        );
        assert!(!app
            .ecr_state
            .scan
            .view
            .as_ref()
            .unwrap()
            .data()
            .visible_column_ids
            .contains(&last));
        close_preferences(&mut app);
        // Header changes prove rendering uses the very same column selection.
        let text = screen(&app, 240);
        assert!(
            !text.contains(last),
            "hidden {last} still rendered on {tab_kind:?}"
        );
        open_preferences(&mut app);
        let prefs = model(&app);
        let page = prefs
            .rows()
            .iter()
            .position(|row| {
                matches!(
                    row,
                    PreferenceRow::PageSize {
                        value: PageSize::Ten,
                        ..
                    }
                )
            })
            .unwrap();
        {
            let table = &mut app.ecr_state.scan.view.as_mut().unwrap().data_mut().table;
            table.selected = 55;
            table.scroll_offset = 50;
            table.expanded_item = Some(55);
        }
        toggle(&mut app, page);
        let view = app.ecr_state.scan.view.as_ref().unwrap();
        assert_eq!(view.data().table.page_size, PageSize::Ten);
        assert_eq!(view.data().table.selected, 0);
        assert!(view.data().table.expanded_item.is_none());
        assert_eq!(app.ecr_image_visible_column_ids, parent_columns);
        assert_eq!(app.ecr_state.images.page_size, parent_page_size);
        assert!(!app.ecr_state.scan.busy());
        assert!(!app.ecr_state.repositories.loading);
    }
}

#[test]
fn last_visible_column_is_protected_on_every_image_table() {
    for tab_kind in TABLE_TABS {
        let mut app = loaded_table(tab_kind);
        open_preferences(&mut app);
        let indices: Vec<_> = model(&app)
            .rows()
            .iter()
            .enumerate()
            .filter_map(|(i, row)| matches!(row, PreferenceRow::Column(_)).then_some(i))
            .collect();
        for index in indices {
            toggle(&mut app, index);
        }
        assert_eq!(model(&app).columns().filter(|c| c.visible).count(), 1);
        assert_eq!(
            app.ecr_state
                .scan
                .view
                .as_ref()
                .unwrap()
                .data()
                .visible_column_ids
                .len(),
            1
        );
    }
}

#[test]
fn tab_preferences_are_independent_and_survive_refresh_and_source_changes() {
    let mut app = loaded_table(DetailTab::Replication);
    open_preferences(&mut app);
    let index = model(&app)
        .rows()
        .iter()
        .position(|row| matches!(row, PreferenceRow::Column(c) if c.id == "Error"))
        .unwrap();
    toggle(&mut app, index);
    let index = model(&app)
        .rows()
        .iter()
        .position(|row| {
            matches!(
                row,
                PreferenceRow::PageSize {
                    value: PageSize::Ten,
                    ..
                }
            )
        })
        .unwrap();
    toggle(&mut app, index);
    close_preferences(&mut app);
    let expected = app
        .ecr_state
        .scan
        .view
        .as_ref()
        .unwrap()
        .data()
        .visible_column_ids
        .clone();
    tab(&mut app);
    assert_eq!(
        app.ecr_state.scan.view.as_ref().unwrap().tab,
        DetailTab::Signing
    );
    assert_eq!(
        model(&app).column_count(),
        DetailTab::Signing.columns().len()
    );
    assert_eq!(
        app.ecr_state
            .scan
            .view
            .as_ref()
            .unwrap()
            .data()
            .table
            .page_size,
        PageSize::Fifty
    );
    press(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
    press(&mut app, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert_eq!(
        app.ecr_state
            .scan
            .view
            .as_ref()
            .unwrap()
            .data()
            .visible_column_ids,
        expected
    );
    assert_eq!(
        app.ecr_state
            .scan
            .view
            .as_ref()
            .unwrap()
            .data()
            .table
            .page_size,
        PageSize::Ten
    );
    app.ecr_state.scan.view.as_mut().unwrap().metadata = Some(details());
    press(&mut app, KeyCode::Char('s'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    enter(&mut app);
    assert_eq!(
        app.ecr_state
            .scan
            .view
            .as_ref()
            .unwrap()
            .data()
            .visible_column_ids,
        expected
    );
    assert_eq!(
        app.ecr_state
            .scan
            .view
            .as_ref()
            .unwrap()
            .data()
            .table
            .page_size,
        PageSize::Ten
    );
}

#[test]
fn general_information_has_no_table_preferences_and_parent_preferences_still_work() {
    let mut app = app();
    enter(&mut app);
    press(&mut app, KeyCode::Char('p'), KeyModifiers::NONE);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(model(&app).column_count(), 0);
    assert!(!model(&app).is_enabled());
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    open_preferences(&mut app);
    assert_eq!(
        model(&app).column_count(),
        crate::ecr::image::Column::all().len()
    );
    close_preferences(&mut app);
    press(&mut app, KeyCode::Esc, KeyModifiers::NONE);
    open_preferences(&mut app);
    assert_eq!(model(&app).column_count(), app.ecr_repo_column_ids.len());
}

#[tokio::test]
async fn yank_copies_exact_image_uri_on_every_tab_including_untagged_images() {
    let mut app = app();
    enter(&mut app);
    finish(&mut app, Ok(Output::Metadata(details()))).await;
    let empty = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 1, 1));
    for untagged in [false, true] {
        if untagged {
            app.ecr_state
                .scan
                .view
                .as_mut()
                .unwrap()
                .metadata
                .as_mut()
                .unwrap()
                .tags
                .clear();
        }
        let expected = if untagged {
            "123456789012.dkr.ecr.us-east-1.amazonaws.com/nested/repo@sha256:0"
        } else {
            "123456789012.dkr.ecr.us-east-1.amazonaws.com/nested/repo:v2-cargo"
        };
        for tab in DetailTab::ALL {
            app.ecr_state.scan.activate(*tab, false);
            press(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
            assert!(app.clipboard.complete_with(&empty, |text| {
                assert_eq!(text, expected);
                Ok(())
            }));
            assert!(!app.clipboard.snapshot_requested());
        }
    }
}

#[test]
fn snapshot_shortcut_is_advertised_and_works_on_every_image_tab() {
    for tab_kind in DetailTab::ALL {
        let mut app = app();
        enter(&mut app);
        app.ecr_state.scan.activate(*tab_kind, false);
        assert!(
            screen(&app, 240).contains("^p print"),
            "missing print hint on {tab_kind:?}"
        );
        press(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert!(app.clipboard.snapshot_requested());
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.ecr_state.scan.view.as_ref().unwrap().tab, *tab_kind);
    }
}
