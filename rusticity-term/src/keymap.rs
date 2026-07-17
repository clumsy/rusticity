use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    SpaceMenu,
    ServicePicker,
    ColumnSelector,
    FilterInput,
    EventFilterInput,
    InsightsInput,
    ErrorModal,
    HelpModal,
    RegionPicker,
    ProfilePicker,
    CalendarPicker,
    TabPicker,
    SessionPicker,
    QuitConfirm,
}

pub fn handle_key(key: KeyEvent, mode: Mode) -> Option<Action> {
    match mode {
        Mode::Normal => match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::ConfirmQuit)
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::CloseService)
            }
            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::OpenInConsole)
            }
            KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => Some(Action::Refresh),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::PageUp)
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::PageDown)
            }
            KeyCode::Esc => Some(Action::GoBack),
            KeyCode::Char('i') => Some(Action::StartFilter),
            KeyCode::Char('c') => Some(Action::OpenCalendar),
            KeyCode::Down => Some(Action::NextItem),
            KeyCode::Up => Some(Action::PrevItem),
            KeyCode::Right => Some(Action::ExpandRow),
            KeyCode::Left => Some(Action::CollapseRow),
            KeyCode::Tab => Some(Action::NextDetailTab),
            KeyCode::BackTab => Some(Action::PrevDetailTab),
            KeyCode::Enter => Some(Action::Select),
            KeyCode::Char(' ') => Some(Action::OpenSpaceMenu),
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::CopyToClipboard)
            }
            KeyCode::Char('p') => Some(Action::OpenColumnSelector),
            KeyCode::Char('e') => Some(Action::ToggleExactMatch),
            KeyCode::Char('x') => Some(Action::ToggleShowExpired),
            KeyCode::Char('s') => Some(Action::CycleSortColumn),
            KeyCode::Char('o') => Some(Action::ToggleSortDirection),
            KeyCode::Char('y') => Some(Action::Yank),
            KeyCode::Char('[') => Some(Action::PrevTab),
            KeyCode::Char(']') => Some(Action::NextTab),
            KeyCode::Char('?') => Some(Action::ShowHelp),
            KeyCode::Char('N')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                Some(Action::NextTab)
            }
            KeyCode::Char('P')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                Some(Action::PrevTab)
            }
            KeyCode::Char(c) if c.is_ascii_digit() => Some(Action::FilterInput(c)),
            KeyCode::Char('P') => Some(Action::ApplyFilter),
            _ => None,
        },
        Mode::SpaceMenu => match key.code {
            KeyCode::Esc => Some(Action::CloseMenu),
            KeyCode::Char('o') => Some(Action::OpenServicePicker),
            KeyCode::Char('r') => Some(Action::OpenRegionPicker),
            KeyCode::Char('p') => Some(Action::OpenProfilePicker),
            KeyCode::Char('a') => Some(Action::OpenCloudWatchAlarms),
            KeyCode::Char('c') => Some(Action::CloseService),
            KeyCode::Char('b') | KeyCode::Char('t') => Some(Action::OpenTabPicker),
            KeyCode::Char('s') => Some(Action::OpenSessionPicker),
            KeyCode::Char('h') => Some(Action::ShowHelp),
            _ => None,
        },
        Mode::ServicePicker => match key.code {
            KeyCode::Esc => Some(Action::ExitFilterMode),
            KeyCode::Char('i') if key.modifiers.is_empty() => Some(Action::EnterFilterMode),
            KeyCode::Down => Some(Action::NextItem),
            KeyCode::Up => Some(Action::PrevItem),
            KeyCode::Enter => Some(Action::Select),
            // 'q' quits when filter is not active; when filter is active it types 'q'
            // We can't check filter_active here, so we always emit FilterInput('q')
            // and handle quit-from-picker in the Action::Quit handler (Normal mode only).
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::DeleteWord)
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => Some(Action::WordLeft),
            KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => Some(Action::WordRight),
            KeyCode::Char(c) if c != 'i' => Some(Action::FilterInput(c)),
            KeyCode::Backspace => Some(Action::FilterBackspace),
            _ => None,
        },
        Mode::ColumnSelector => match key.code {
            KeyCode::Esc => Some(Action::CloseColumnSelector),
            KeyCode::Down => Some(Action::NextItem),
            KeyCode::Up => Some(Action::PrevItem),
            KeyCode::Char(' ') | KeyCode::Enter => Some(Action::ToggleColumn),
            KeyCode::Tab => Some(Action::NextPreferences),
            KeyCode::BackTab => Some(Action::PrevPreferences),
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::PageDown)
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::PageUp)
            }
            _ => None,
        },
        Mode::FilterInput => match key.code {
            KeyCode::Esc => Some(Action::CloseMenu),
            KeyCode::Enter => Some(Action::ApplyFilter),
            KeyCode::BackTab => Some(Action::PrevFilterFocus),
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(Action::PrevFilterFocus)
            }
            KeyCode::Tab => Some(Action::NextFilterFocus),
            KeyCode::Up => Some(Action::PrevItem),
            KeyCode::Down => Some(Action::NextItem),
            KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => Some(Action::WordLeft),
            KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => Some(Action::WordRight),
            KeyCode::Left => Some(Action::PageUp),
            KeyCode::Right => Some(Action::PageDown),
            KeyCode::Char(' ') => Some(Action::ToggleFilterCheckbox),
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::DeleteWord)
            }
            KeyCode::Char(c) if c != ' ' => Some(Action::FilterInput(c)),
            KeyCode::Backspace => Some(Action::FilterBackspace),
            _ => None,
        },
        Mode::EventFilterInput => match key.code {
            KeyCode::Esc => Some(Action::CloseMenu),
            KeyCode::Enter => Some(Action::ApplyFilter),
            KeyCode::BackTab => Some(Action::PrevFilterFocus),
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                Some(Action::PrevFilterFocus)
            }
            KeyCode::Tab => Some(Action::NextFilterFocus),
            KeyCode::Char(' ') => Some(Action::ToggleFilterCheckbox),
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::DeleteWord)
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => Some(Action::WordLeft),
            KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => Some(Action::WordRight),
            KeyCode::Char(c) if c != ' ' => Some(Action::FilterInput(c)),
            KeyCode::Backspace => Some(Action::FilterBackspace),
            _ => None,
        },
        Mode::InsightsInput => match key.code {
            KeyCode::Esc => Some(Action::CloseMenu),
            KeyCode::Enter => Some(Action::Select),
            KeyCode::Tab => Some(Action::NextFilterFocus),
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::Refresh)
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::DeleteWord)
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => Some(Action::WordLeft),
            KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => Some(Action::WordRight),
            KeyCode::Down => Some(Action::NextItem),
            KeyCode::Up => Some(Action::PrevItem),
            KeyCode::Char(' ') => Some(Action::ToggleFilterCheckbox),
            KeyCode::Char(c) if c != ' ' => Some(Action::FilterInput(c)),
            KeyCode::Backspace => Some(Action::FilterBackspace),
            _ => None,
        },
        Mode::ErrorModal => match key.code {
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::RetryLoad)
            }
            KeyCode::Char('y') => Some(Action::Yank),
            KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
            _ => None,
        },
        Mode::HelpModal => match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('?') => {
                Some(Action::CloseMenu)
            }
            _ => None,
        },
        Mode::QuitConfirm => match key.code {
            KeyCode::Char('y') | KeyCode::Enter => Some(Action::ConfirmQuit),
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => Some(Action::CancelQuit),
            _ => None,
        },
        Mode::RegionPicker => match key.code {
            KeyCode::Esc => Some(Action::ExitFilterMode),
            KeyCode::Char('i') => Some(Action::EnterFilterMode),
            KeyCode::Char('j') | KeyCode::Down => Some(Action::NextItem),
            KeyCode::Char('k') | KeyCode::Up => Some(Action::PrevItem),
            KeyCode::Enter => Some(Action::Select),
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Action::MeasureLatency)
            }
            KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => Some(Action::Refresh),
            KeyCode::Backspace => Some(Action::FilterBackspace),
            KeyCode::Char(c) => Some(Action::FilterInput(c)),
            _ => None,
        },
        Mode::CalendarPicker => match key.code {
            KeyCode::Esc => Some(Action::CloseCalendar),
            KeyCode::Left => Some(Action::CalendarPrevDay),
            KeyCode::Down => Some(Action::CalendarNextWeek),
            KeyCode::Up => Some(Action::CalendarPrevWeek),
            KeyCode::Right => Some(Action::CalendarNextDay),
            KeyCode::Char('n') | KeyCode::Tab => Some(Action::CalendarNextMonth),
            KeyCode::Char('p') | KeyCode::BackTab => Some(Action::CalendarPrevMonth),
            KeyCode::Enter => Some(Action::CalendarSelect),
            _ => None,
        },
        Mode::TabPicker => match key.code {
            KeyCode::Esc => Some(Action::CloseMenu),
            KeyCode::Down => Some(Action::NextItem),
            KeyCode::Up => Some(Action::PrevItem),
            KeyCode::Enter => Some(Action::Select),
            KeyCode::Backspace => Some(Action::FilterBackspace),
            KeyCode::Char(c) => Some(Action::FilterInput(c)),
            _ => None,
        },
        Mode::SessionPicker => match key.code {
            KeyCode::Esc => Some(Action::ExitFilterMode),
            KeyCode::Char('i') => Some(Action::EnterFilterMode),
            KeyCode::Down => Some(Action::NextItem),
            KeyCode::Up => Some(Action::PrevItem),
            KeyCode::Enter => Some(Action::LoadSession),
            KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => Some(Action::Refresh),
            KeyCode::Backspace => Some(Action::FilterBackspace),
            KeyCode::Char(c) => Some(Action::FilterInput(c)),
            _ => None,
        },
        Mode::ProfilePicker => match key.code {
            KeyCode::Esc => Some(Action::ExitFilterMode),
            KeyCode::Char('i') => Some(Action::EnterFilterMode),
            KeyCode::Down => Some(Action::NextItem),
            KeyCode::Up => Some(Action::PrevItem),
            KeyCode::Enter => Some(Action::Select),
            KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => Some(Action::Refresh),
            KeyCode::Backspace => Some(Action::FilterBackspace),
            KeyCode::Char(c) => Some(Action::FilterInput(c)),
            _ => None,
        },
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    ConfirmQuit,
    CancelQuit,
    CloseService,
    NextItem,
    PrevItem,
    NextPane,
    PrevPane,
    CollapseRow,
    ExpandRow,
    Select,
    OpenSpaceMenu,
    CloseMenu,
    OpenServicePicker,
    OpenCloudWatch,
    OpenCloudWatchSplit,
    OpenCloudWatchAlarms,
    FilterInput(char),
    FilterBackspace,
    DeleteWord,
    WordLeft,
    WordRight,
    OpenColumnSelector,
    ToggleColumn,
    NextPreferences,
    PrevPreferences,
    CloseColumnSelector,
    StartFilter,
    StartEventFilter,
    ApplyFilter,
    ToggleExactMatch,
    ToggleShowExpired,
    GoBack,
    NextFilterFocus,
    PrevFilterFocus,
    ToggleFilterCheckbox,
    CycleSortColumn,
    ToggleSortDirection,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    Refresh,
    MeasureLatency,
    RetryLoad,
    Yank,
    OpenInConsole,
    OpenInBrowser,
    ShowHelp,
    OpenRegionPicker,
    OpenCalendar,
    CloseCalendar,
    CalendarPrevDay,
    CalendarNextDay,
    CalendarPrevWeek,
    CalendarNextWeek,
    CalendarPrevMonth,
    CalendarNextMonth,
    CalendarSelect,
    NextTab,
    PrevTab,
    NextDetailTab,
    PrevDetailTab,
    CloseTab,
    OpenTabPicker,
    OpenSessionPicker,
    OpenProfilePicker,
    LoadSession,
    SaveSession,
    CopyToClipboard,
    EnterFilterMode,
    ExitFilterMode,
    Noop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_space_o_opens_service_menu() {
        let key = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
        let action = handle_key(key, Mode::SpaceMenu);
        assert_eq!(action, Some(Action::OpenServicePicker));
    }

    #[test]
    fn test_insights_input_accepts_chars() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let action = handle_key(key, Mode::InsightsInput);
        assert_eq!(action, Some(Action::FilterInput('a')));

        let key2 = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
        let action2 = handle_key(key2, Mode::InsightsInput);
        assert_eq!(action2, Some(Action::FilterInput('1')));
    }

    #[test]
    fn test_insights_input_esc_closes() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let action = handle_key(key, Mode::InsightsInput);
        assert_eq!(action, Some(Action::CloseMenu));
    }

    #[test]
    fn test_service_menu_accepts_input() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        let action = handle_key(key, Mode::ServicePicker);
        assert_eq!(action, Some(Action::FilterInput('c')));
    }

    #[test]
    fn test_service_menu_navigation() {
        let key_down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let action_down = handle_key(key_down, Mode::ServicePicker);
        assert_eq!(action_down, Some(Action::NextItem));

        let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let action_up = handle_key(key_up, Mode::ServicePicker);
        assert_eq!(action_up, Some(Action::PrevItem));
    }

    #[test]
    fn test_service_menu_backspace() {
        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        let action = handle_key(key, Mode::ServicePicker);
        assert_eq!(action, Some(Action::FilterBackspace));
    }

    #[test]
    fn test_ctrl_shift_n_next_tab() {
        let key = KeyEvent::new(
            KeyCode::Char('N'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        let action = handle_key(key, Mode::Normal);
        assert_eq!(action, Some(Action::NextTab));
    }

    #[test]
    fn test_ctrl_shift_p_prev_tab() {
        let key = KeyEvent::new(
            KeyCode::Char('P'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        let action = handle_key(key, Mode::Normal);
        assert_eq!(action, Some(Action::PrevTab));
    }

    #[test]
    fn test_space_c_close_tab() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        let action = handle_key(key, Mode::SpaceMenu);
        assert_eq!(action, Some(Action::CloseService));
    }

    #[test]
    fn test_space_b_window_picker() {
        let key = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);
        let action = handle_key(key, Mode::SpaceMenu);
        assert_eq!(action, Some(Action::OpenTabPicker));
    }

    #[test]
    fn test_window_picker_navigation() {
        let key_down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let action = handle_key(key_down, Mode::TabPicker);
        assert_eq!(action, Some(Action::NextItem));

        let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let action_up = handle_key(key_up, Mode::TabPicker);
        assert_eq!(action_up, Some(Action::PrevItem));
    }

    #[test]
    fn test_window_picker_select() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = handle_key(key, Mode::TabPicker);
        assert_eq!(action, Some(Action::Select));
    }

    #[test]
    fn test_space_opens_space_menu_in_normal_mode() {
        let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let action = handle_key(key, Mode::Normal);
        assert_eq!(action, Some(Action::OpenSpaceMenu));
    }

    #[test]
    fn test_space_menu_o_opens_service_menu() {
        let key = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
        let action = handle_key(key, Mode::SpaceMenu);
        assert_eq!(action, Some(Action::OpenServicePicker));
    }

    #[test]
    fn test_ctrl_r_refreshes_profile_picker() {
        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        let action = handle_key(key, Mode::ProfilePicker);
        assert_eq!(action, Some(Action::Refresh));
    }

    #[test]
    fn test_ctrl_r_refreshes_region_picker() {
        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        let action = handle_key(key, Mode::RegionPicker);
        assert_eq!(action, Some(Action::Refresh));
    }

    #[test]
    fn test_ctrl_r_refreshes_session_picker() {
        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        let action = handle_key(key, Mode::SessionPicker);
        assert_eq!(action, Some(Action::Refresh));
    }

    #[test]
    fn test_p_opens_column_selector() {
        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
        let action = handle_key(key, Mode::Normal);
        assert_eq!(
            action,
            Some(Action::OpenColumnSelector),
            "p should open column selector (preferences)"
        );
    }

    #[test]
    fn test_ctrl_p_copies_to_clipboard() {
        let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        let action = handle_key(key, Mode::Normal);
        assert_eq!(
            action,
            Some(Action::CopyToClipboard),
            "Ctrl+P should copy screen to clipboard (print)"
        );
    }

    #[test]
    fn test_y_yanks_selected_item() {
        let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
        let action = handle_key(key, Mode::Normal);
        assert_eq!(
            action,
            Some(Action::Yank),
            "y should yank (copy) selected item"
        );
    }

    #[test]
    fn test_space_toggles_checkbox_in_filter_input() {
        let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let action = handle_key(key, Mode::FilterInput);
        assert_eq!(
            action,
            Some(Action::ToggleFilterCheckbox),
            "Space should toggle checkbox in FilterInput mode"
        );
    }

    #[test]
    fn test_space_not_added_to_filter_text() {
        // Space should toggle checkbox, not be added to filter text
        let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let action = handle_key(key, Mode::FilterInput);
        assert_ne!(
            action,
            Some(Action::FilterInput(' ')),
            "Space should not be added to filter text"
        );
    }

    #[test]
    fn test_q_in_normal_mode_shows_quit_confirmation() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let action = handle_key(key, Mode::Normal);
        assert_eq!(
            action,
            Some(Action::Quit),
            "q in Normal mode must trigger Quit (shows confirmation dialog)"
        );
    }

    #[test]
    fn test_ctrl_c_quits_immediately_without_confirmation() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let action = handle_key(key, Mode::Normal);
        assert_eq!(
            action,
            Some(Action::ConfirmQuit),
            "Ctrl+C must quit immediately (ConfirmQuit, not Quit)"
        );
    }

    #[test]
    fn test_q_in_service_picker_shows_quit_confirmation() {
        // 'q' in ServicePicker now dispatches FilterInput('q') so it can type 'q'
        // when filter is active (e.g. typing "sqs"). When filter is NOT active,
        // app.rs handles FilterInput('q') as QuitConfirm.
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let action = handle_key(key, Mode::ServicePicker);
        assert_eq!(
            action,
            Some(Action::FilterInput('q')),
            "q in ServicePicker must dispatch FilterInput so it can type 'sqs' in filter"
        );
    }

    #[test]
    fn test_q_in_service_picker_not_filtering_triggers_quit() {
        // When ServicePicker filter is NOT active, FilterInput('q') triggers QuitConfirm
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.mode = Mode::ServicePicker;
        app.service_picker.filter_active = false; // not filtering

        app.handle_action(Action::FilterInput('q'));

        assert_eq!(
            app.mode,
            Mode::QuitConfirm,
            "q when not filtering in ServicePicker must show quit confirmation"
        );
    }

    #[test]
    fn test_q_in_service_picker_while_filtering_types_q() {
        // When ServicePicker filter IS active (user is typing), 'q' types into filter
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.mode = Mode::ServicePicker;
        app.service_picker.filter_active = true; // filter is active (user pressed 'i')

        app.handle_action(Action::FilterInput('q'));

        assert_eq!(
            app.mode,
            Mode::ServicePicker,
            "q while filtering must NOT quit"
        );
        assert_eq!(
            app.service_picker.filter, "q",
            "q while filtering must type 'q' into filter"
        );
    }

    #[test]
    fn test_quit_confirm_mode_y_confirms() {
        let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
        let action = handle_key(key, Mode::QuitConfirm);
        assert_eq!(action, Some(Action::ConfirmQuit));
    }

    #[test]
    fn test_quit_confirm_mode_enter_confirms() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = handle_key(key, Mode::QuitConfirm);
        assert_eq!(action, Some(Action::ConfirmQuit));
    }

    #[test]
    fn test_quit_confirm_mode_n_cancels() {
        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        let action = handle_key(key, Mode::QuitConfirm);
        assert_eq!(action, Some(Action::CancelQuit));
    }

    #[test]
    fn test_quit_confirm_mode_esc_cancels() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let action = handle_key(key, Mode::QuitConfirm);
        assert_eq!(action, Some(Action::CancelQuit));
    }

    #[test]
    fn test_quit_shows_confirmation_dialog_not_immediate_quit() {
        use crate::app::{App, Service};
        let mut app = App::new_without_client("default".to_string(), None);
        app.current_service = Service::CloudWatchLogGroups;
        app.service_selected = true;
        app.mode = Mode::Normal;

        app.handle_action(Action::Quit);

        assert_eq!(
            app.mode,
            Mode::QuitConfirm,
            "q must show confirmation dialog"
        );
        assert!(
            app.running,
            "app must still be running after q (before confirmation)"
        );
    }

    #[test]
    fn test_confirm_quit_actually_quits() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.mode = Mode::QuitConfirm;

        app.handle_action(Action::ConfirmQuit);

        assert!(!app.running, "ConfirmQuit must set running=false");
    }

    #[test]
    fn test_cancel_quit_returns_to_normal() {
        use crate::app::App;
        let mut app = App::new_without_client("default".to_string(), None);
        app.mode = Mode::QuitConfirm;

        app.handle_action(Action::CancelQuit);

        assert_eq!(
            app.mode,
            Mode::Normal,
            "CancelQuit must return to Normal mode"
        );
        assert!(app.running, "app must still be running after cancel");
    }
}
