use agcodex_core::protocol::Event;
use agcodex_core::subagents::{InvocationRequest, SubagentExecution, SubagentStatus};
use agcodex_file_search::FileMatch;
use agcodex_persistence::types::SessionMetadata;
use ratatui::crossterm::event::KeyEvent;
use ratatui::text::Line;
use std::time::Duration;
use uuid::Uuid;

use crate::app::ChatWidgetArgs;
use crate::slash_command::SlashCommand;
// Note: These imports will be used when session browser event handling is implemented
#[allow(unused_imports)]
use crate::widgets::{SessionAction, ViewMode, SortBy, FocusedPanel};
use agcodex_core::protocol::AskForApproval;
use agcodex_core::protocol::SandboxPolicy;
use agcodex_core::protocol_config_types::ReasoningEffort;

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub(crate) enum AppEvent {
    CodexEvent(Event),

    /// Request a redraw which will be debounced by the [`App`].
    RequestRedraw,

    /// Actually draw the next frame.
    Redraw,

    /// Schedule a one-shot animation frame roughly after the given duration.
    /// Multiple requests are coalesced by the central frame scheduler.
    ScheduleFrameIn(Duration),

    KeyEvent(KeyEvent),

    /// Text pasted from the terminal clipboard.
    Paste(String),

    /// Request to exit the application gracefully.
    ExitRequest,

    /// Forward an `Op` to the Agent. Using an `AppEvent` for this avoids
    /// bubbling channels through layers of widgets.
    CodexOp(agcodex_core::protocol::Op),

    /// Dispatch a recognized slash command from the UI (composer) to the app
    /// layer so it can be handled centrally.
    DispatchCommand(SlashCommand),

    /// Kick off an asynchronous file search for the given query (text after
    /// the `@`). Previous searches may be cancelled by the app layer so there
    /// is at most one in-flight search.
    StartFileSearch(String),

    /// Result of a completed asynchronous file search. The `query` echoes the
    /// original search term so the UI can decide whether the results are
    /// still relevant.
    FileSearchResult {
        query: String,
        matches: Vec<FileMatch>,
    },

    /// Result of computing a `/diff` command.
    DiffResult(String),

    InsertHistory(Vec<Line<'static>>),

    StartCommitAnimation,
    StopCommitAnimation,
    CommitTick,

    /// Onboarding: result of login_with_chatgpt.
    OnboardingAuthComplete(Result<(), String>),
    OnboardingComplete(ChatWidgetArgs),

    /// Update the current reasoning effort in the running app and widget.
    UpdateReasoningEffort(ReasoningEffort),

    /// Update the current model slug in the running app and widget.
    UpdateModel(String),

    /// Update the current approval policy in the running app and widget.
    UpdateAskForApprovalPolicy(AskForApproval),

    /// Update the current sandbox policy in the running app and widget.
    UpdateSandboxPolicy(SandboxPolicy),

    /// Cycle to the next operating mode (Plan → Build → Review → Plan).
    CycleModes,

    /// Request to show the message jump popup.
    ShowMessageJump,

    /// Request to hide the message jump popup.
    HideMessageJump,

    /// Jump to a specific message index in the conversation history.
    JumpToMessage(usize),

    /// Update search query in message jump popup.
    MessageJumpSearch(String),

    /// Cycle role filter in message jump popup.
    MessageJumpCycleFilter,

    /// Open the save session dialog.
    OpenSaveDialog,

    /// Save session with the provided name and description.
    SaveSession {
        name: String,
        description: Option<String>,
    },

    /// Close the save dialog.
    CloseSaveDialog,

    /// Open the load session dialog
    OpenLoadDialog,

    /// Close the load session dialog
    CloseLoadDialog,

    /// Start loading session list for the load dialog
    StartLoadSessionList,

    /// Result of loading session list
    LoadSessionListResult(Result<Vec<SessionMetadata>, String>),

    /// Load a specific session by ID
    LoadSession(Uuid),

    /// Result of loading a session
    LoadSessionResult(Result<Uuid, String>),

    /// Update search query in load dialog
    UpdateLoadDialogQuery(String),

    /// Open the session browser (Ctrl+H)
    OpenSessionBrowser,

    /// Close the session browser
    CloseSessionBrowser,

    /// Session browser: navigate up/down
    SessionBrowserNavigate(i32),

    /// Session browser: change panel focus
    SessionBrowserFocusNext,
    SessionBrowserFocusPrevious,

    /// Session browser: toggle view mode
    SessionBrowserToggleViewMode,

    /// Session browser: cycle sort order
    SessionBrowserCycleSort,

    /// Session browser: update search query
    SessionBrowserUpdateSearch(String),

    /// Session browser: execute selected action
    SessionBrowserExecuteAction,

    /// Session browser: delete session
    SessionBrowserDeleteSession(Uuid),

    /// Session browser: export session
    SessionBrowserExportSession { id: Uuid, format: String },

    /// Session browser: rename session
    SessionBrowserRenameSession { id: Uuid, new_name: String },

    /// Session browser: toggle favorite
    SessionBrowserToggleFavorite(Uuid),

    /// Session browser: duplicate session
    SessionBrowserDuplicateSession(Uuid),

    /// Session browser: show confirmation dialog
    SessionBrowserShowConfirmation(String),

    /// Session browser: confirm action
    SessionBrowserConfirmAction(bool),

    /// Session browser: toggle favorites filter
    SessionBrowserToggleFavoritesFilter,

    /// Session browser: toggle expand
    SessionBrowserToggleExpand,

    /// Session browser: select item
    SessionBrowserSelect,

    /// Session browser: delete item
    SessionBrowserDelete,

    /// Session browser: filter
    SessionBrowserFilter(String),

    /// Session browser: sort
    SessionBrowserSort(String),

    /// Start history get operation
    StartHistoryGet,

    /// History get result
    HistoryGetResult(String),

    /// Start jump to message operation
    StartJumpToMessage(usize),

    /// Start undo operation
    StartUndo,

    /// Undo complete
    UndoComplete,

    /// Start redo operation
    StartRedo,

    /// Redo complete
    RedoComplete,

    /// Start fork operation
    StartFork,

    /// Fork complete
    ForkComplete(String),

    // ===== Agent Events =====
    /// Start agent execution from invocation request
    StartAgent(InvocationRequest),

    /// Agent execution progress update
    AgentProgress {
        agent_id: Uuid,
        progress: f32,        // 0.0 to 1.0
        message: String,      // Current status message
    },

    /// Agent execution completed
    AgentComplete {
        agent_id: Uuid,
        execution: SubagentExecution,
    },

    /// Agent execution failed
    AgentFailed {
        agent_id: Uuid,
        error: String,
    },

    /// Cancel running agent
    CancelAgent(Uuid),

    /// Toggle agent panel visibility (Ctrl+A)
    ToggleAgentPanel,

    /// Agent panel navigation events
    AgentPanelNavigateUp,
    AgentPanelNavigateDown,
    AgentPanelCancel,
    
    /// Agent output chunk received (for streaming)
    AgentOutputChunk {
        agent_id: Uuid,
        chunk: String,
    },
}
