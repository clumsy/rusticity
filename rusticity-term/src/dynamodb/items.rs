//! Explicit, bounded Items requests. Local table pages never advance an AWS cursor.
use super::actions::is_items_tab;
use crate::app::{App, Service};
use crate::keymap::Mode;
use crate::ui::dynamodb::{self, ItemOperation, ItemProjection, State};
use rusticity_core::dynamodb::{DdbItemCursor, DdbItemsPage};

#[derive(Clone, Debug, PartialEq)]
pub struct ItemRequestContext {
    pub profile: String,
    pub region: String,
    pub account_id: String,
    pub role_arn: String,
    pub table: String,
    pub index: Option<String>,
    pub keys: Vec<String>,
    pub partition_type: String,
    pub operation: ItemOperation,
    pub projection: ItemProjection,
    /// Scan text is a local filter, not part of the AWS request context.
    pub query: Option<String>,
}

pub fn context(app: &App) -> Option<ItemRequestContext> {
    let table = dynamodb::current_table(app)?;
    let state = &app.dynamodb_state;
    Some(ItemRequestContext {
        profile: app.profile.clone(),
        region: app.config.region.clone(),
        account_id: app.config.account_id.clone(),
        role_arn: app.config.role_arn.clone(),
        table: table.name.clone(),
        index: dynamodb::selected_item_index(table, state.item_index),
        keys: dynamodb::selected_key_attributes(table, state.item_index),
        partition_type: dynamodb::key_attribute_type(&dynamodb::selected_partition_key_label(
            table,
            state.item_index,
        )),
        operation: state.item_operation,
        projection: state.item_projection,
        query: (state.item_operation == ItemOperation::Query).then(|| state.item_filter.clone()),
    })
}

pub fn can_load_more(app: &App) -> bool {
    let state = &app.dynamodb_state;
    app.current_service == Service::DynamoDbTables
        && is_items_tab(app)
        && !state.items_loading
        && state.items_cursor.is_some()
        && state.items_context.is_some()
        && state.items_context == context(app)
}

/// A finite total is known only after a successful request for the current
/// context exhausts its cursor. A disabled Load more action is not proof of this.
pub fn results_complete(app: &App) -> bool {
    let state = &app.dynamodb_state;
    !state.items_loading
        && !state.items_failed
        && state.items_cursor.is_none()
        && state.items_context.is_some()
        && state.items_context == context(app)
        && (state.item_operation != ItemOperation::Query || !state.item_filter.is_empty())
}

pub fn needs_load(app: &App) -> bool {
    let state = &app.dynamodb_state;
    if !is_items_tab(app) || app.mode == Mode::ErrorModal {
        return false;
    }
    if state.items_loading {
        return true;
    }
    if app.mode == Mode::FilterInput {
        return false;
    }
    // A context change restarts even when the Query key was cleared. The loader
    // clears the previous results but sends no request for an empty key.
    if state.items_context.is_some() && state.items_context != context(app) {
        return true;
    }
    state.item_operation == ItemOperation::Query
        && !state.item_filter.is_empty()
        && state.executed_query != state.item_filter
}

pub fn load_more(app: &mut App) {
    if app.mode == Mode::Normal && can_load_more(app) {
        app.dynamodb_state.items_append = true;
        app.dynamodb_state.items_loading = true;
    }
}

pub fn retry(app: &mut App) {
    // Keep the failed request's append/replace intent. begin_items_request still
    // rejects the old cursor if the account, table, or query context has changed.
    app.dynamodb_state.items_loading = true;
}

impl State {
    pub fn clear_items_result(&mut self) {
        self.items.reset();
        self.items.items.clear();
        self.items_cursor = None;
        self.items_context = None;
        self.items_append = false;
        self.items_failed = false;
        self.items_loading = false;
        self.executed_query.clear();
    }

    pub fn begin_items_request(&mut self, context: ItemRequestContext) -> Option<DdbItemCursor> {
        self.items_append = self.items_append
            && self.items_context.as_ref() == Some(&context)
            && self.items_cursor.is_some();
        if !self.items_append {
            self.items.reset();
            self.items.items.clear();
            self.items_cursor = None;
        }
        // Remember attempted queries too: an error must not automatically retry
        // on every subsequent key press.
        self.executed_query = context.query.clone().unwrap_or_default();
        self.items_context = Some(context);
        self.items_failed = false;
        self.items_loading = true;
        self.items_cursor.clone()
    }

    pub fn finish_items_request(&mut self, page: DdbItemsPage) {
        self.items.items.extend(page.items);
        self.items_cursor = page.next_cursor;
        self.items_loading = false;
        self.items_append = false;
        self.items_failed = false;
    }

    pub fn fail_items_request(&mut self) {
        self.items_loading = false;
        self.items_failed = true;
        // Do not discard rows, cursor, or append intent; Ctrl+R can retry.
    }
}

#[cfg(test)]
mod tests;
