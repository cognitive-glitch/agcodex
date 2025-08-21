//! Load dialog view for the bottom pane

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::user_approval_widget::ApprovalRequest;
use crate::widgets::LoadDialog;
use agcodex_persistence::types::SessionMetadata;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use std::any::Any;

use super::BottomPane;
use super::CancellationEvent;
use super::bottom_pane_view::BottomPaneView;

/// Load dialog view that implements BottomPaneView
pub(crate) struct LoadDialogView {
    dialog: LoadDialog,
    app_event_tx: AppEventSender,
    is_complete: bool,
}

impl LoadDialogView {
    pub(crate) fn new(app_event_tx: AppEventSender) -> Self {
        // Start loading session list immediately
        app_event_tx.send(AppEvent::StartLoadSessionList);

        Self {
            dialog: LoadDialog::new(),
            app_event_tx,
            is_complete: false,
        }
    }

    /// Update the dialog with session list
    pub(crate) fn set_sessions(&mut self, sessions: Vec<SessionMetadata>) {
        self.dialog.set_sessions(sessions);
    }

    /// Set error state
    pub(crate) fn set_error(&mut self, error: String) {
        self.dialog.set_error(error);
    }

    /// Update search query
    pub(crate) fn update_search_query(&mut self, query: String) {
        self.dialog.set_search_query(&query);
    }
}

impl BottomPaneView<'_> for LoadDialogView {
    fn handle_key_event(&mut self, _pane: &mut BottomPane<'_>, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Escape => {
                self.is_complete = true;
                self.app_event_tx.send(AppEvent::CloseLoadDialog);
            }
            KeyCode::Enter => {
                if let Some(session_id) = self.dialog.selected_session_id() {
                    self.is_complete = true;
                    self.app_event_tx.send(AppEvent::LoadSession(session_id));
                }
            }
            KeyCode::Up => {
                self.dialog.move_up();
            }
            KeyCode::Down => {
                self.dialog.move_down();
            }
            KeyCode::Char(c) => {
                // Add character to search query
                let mut query = self.dialog.search_query().to_string();
                query.push(c);
                self.dialog.set_search_query(&query);
                self.app_event_tx
                    .send(AppEvent::UpdateLoadDialogQuery(query));
            }
            KeyCode::Backspace => {
                // Remove character from search query
                let mut query = self.dialog.search_query().to_string();
                query.pop();
                self.dialog.set_search_query(&query);
                self.app_event_tx
                    .send(AppEvent::UpdateLoadDialogQuery(query));
            }
            _ => {
                // Ignore other keys
            }
        }
    }

    fn is_complete(&self) -> bool {
        self.is_complete
    }

    fn on_ctrl_c(&mut self, _pane: &mut BottomPane<'_>) -> CancellationEvent {
        self.is_complete = true;
        self.app_event_tx.send(AppEvent::CloseLoadDialog);
        CancellationEvent::Handled
    }

    fn desired_height(&self, _width: u16) -> u16 {
        self.dialog.calculate_required_height()
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.dialog.render_ref(area, buf);
    }

    fn should_hide_when_task_is_done(&mut self) -> bool {
        false // Don't auto-hide when tasks complete
    }

    fn try_consume_approval_request(
        &mut self,
        request: ApprovalRequest,
    ) -> Option<ApprovalRequest> {
        // Don't consume approval requests - let them be handled normally
        Some(request)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
