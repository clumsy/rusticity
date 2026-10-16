use crate::{
    app::{App, Service},
    inventory::{self, InventoryInput},
    keymap::Action,
    ui::sns::filtered_topics,
};

pub fn selected_arn(app: &App) -> Option<String> {
    let topics = filtered_topics(app);
    app.sns_state
        .topics
        .get_selected(&topics)
        .map(|topic| topic.arn.clone())
}
pub fn console_url(app: &App) -> String {
    if let Some(topic) = &app.sns_state.subscriptions.topic {
        if let Some(subscription) =
            super::subscriptions::selected(app).filter(|s| s.status == "Confirmed")
        {
            return super::console_url(&app.config.region, None)
                .trim_end_matches("#/topics")
                .to_string()
                + "#/subscription/"
                + &urlencoding::encode(&subscription.subscription_arn);
        }
        return super::console_url(&app.config.region, Some(&topic.arn));
    }
    super::console_url(&app.config.region, selected_arn(app).as_deref())
}
pub fn go_to_page(app: &mut App, page: usize) {
    if super::subscriptions::active(app) {
        super::subscriptions::go_to_page(app, page);
        return;
    }
    let count = filtered_topics(app).len();
    inventory::go_to_page(&mut app.sns_state.topics, page, count);
}
pub fn handle_action(app: &mut App, action: &Action) -> bool {
    if !app.service_selected || app.current_service != Service::SnsTopics {
        return false;
    }
    if super::subscriptions::handle_action(app, action) {
        return true;
    }
    if super::subscriptions::active(app) {
        return false;
    }
    let filtered_count = filtered_topics(app).len();
    let selected_arn = selected_arn(app);
    inventory::handle_action(
        InventoryInput {
            table: &mut app.sns_state.topics,
            focus: &mut app.sns_state.input_focus,
            mode: &mut app.mode,
            page_input: &mut app.page_input,
            error_message: &mut app.error_message,
            error_scroll: &mut app.error_scroll,
            filtered_count,
            selected_arn,
            clipboard: &app.clipboard,
        },
        action,
    )
}
