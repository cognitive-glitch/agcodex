use crate::LoginStatus;
use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::chatwidget::ChatWidget;
use crate::file_search::FileSearchManager;
use crate::get_git_diff::get_git_diff;
use crate::get_login_status;
use crate::onboarding::onboarding_screen::KeyboardHandler;
use crate::onboarding::onboarding_screen::OnboardingScreen;
use crate::onboarding::onboarding_screen::OnboardingScreenArgs;
use crate::slash_command::SlashCommand;
use crate::tui;
use agcodex_core::ConversationManager;
use agcodex_core::config::Config;
use agcodex_core::modes::ModeManager;
use agcodex_core::modes::OperatingMode;
use agcodex_core::protocol::Event;
use agcodex_core::protocol::Op;
// Temporarily disable real orchestrator until core compilation issues are fixed
// use agcodex_core::subagents::{
//     AgentOrchestrator, OrchestratorConfig, ProgressUpdate, SubagentRegistry
// };
// use agcodex_core::code_tools::ast_agent_tools::ASTAgentTools;
use color_eyre::eyre::Result;
// Use crossterm types re-exported by ratatui to avoid version conflicts
use crate::widgets::{AgentPanel, ModeIndicator};
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::crossterm::terminal::supports_keyboard_enhancement;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Offset;
use ratatui::prelude::Backend;
use ratatui::text::Line;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::channel;
use tokio::sync::mpsc::UnboundedReceiver;
use uuid::Uuid;
use std::thread;
use std::time::Duration;
use std::time::Instant;

/// Time window for debouncing redraw requests.
const REDRAW_DEBOUNCE: Duration = Duration::from_millis(1);

/// Top-level application state: which full-screen view is currently active.
#[allow(clippy::large_enum_variant)]
enum AppState<'a> {
    Onboarding {
        screen: OnboardingScreen,
    },
    /// The main chat UI is visible.
    Chat {
        /// Boxed to avoid a large enum variant and reduce the overall size of
        /// `AppState`.
        widget: Box<ChatWidget<'a>>,
    },
}

pub(crate) struct App<'a> {
    server: Arc<ConversationManager>,
    app_event_tx: AppEventSender,
    app_event_rx: Receiver<AppEvent>,
    app_state: AppState<'a>,

    /// Config is stored here so we can recreate ChatWidgets as needed.
    config: Config,

    file_search: FileSearchManager,

    pending_history_lines: Vec<Line<'static>>,

    enhanced_keys_supported: bool,

    /// Controls the animation thread that sends CommitTick events.
    commit_anim_running: Arc<AtomicBool>,

    /// Channel to schedule one-shot animation frames; coalesced by a single
    /// scheduler thread.
    frame_schedule_tx: std::sync::mpsc::Sender<Instant>,

    /// Mode manager for operating mode switching
    mode_manager: ModeManager,

    /// Agent panel for subagent management
    agent_panel: AgentPanel,

    // Temporarily disabled until core compilation issues are fixed
    // /// Agent orchestrator for real execution
    // orchestrator: Arc<AgentOrchestrator>,
    //
    // /// AST-based tools for agents
    // ast_tools: Arc<ASTAgentTools>,
    //
    // /// Progress receiver for agent updates
    // progress_receiver: Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<ProgressUpdate>>>,
}

/// Aggregate parameters needed to create a `ChatWidget`, as creation may be
/// deferred until after the Git warning screen is dismissed.
#[derive(Clone, Debug)]
pub(crate) struct ChatWidgetArgs {
    pub(crate) config: Config,
    initial_prompt: Option<String>,
    initial_images: Vec<PathBuf>,
    enhanced_keys_supported: bool,
}

impl App<'_> {
    pub(crate) fn new(
        config: Config,
        initial_prompt: Option<String>,
        initial_images: Vec<std::path::PathBuf>,
        show_trust_screen: bool,
    ) -> Self {
        let conversation_manager = Arc::new(ConversationManager::default());

        let (app_event_tx, app_event_rx) = channel();
        let app_event_tx = AppEventSender::new(app_event_tx);

        let enhanced_keys_supported = supports_keyboard_enhancement().unwrap_or(false);

        // Spawn a dedicated thread for reading the crossterm event loop and
        // re-publishing the events as AppEvents, as appropriate.
        {
            let app_event_tx = app_event_tx.clone();
            std::thread::spawn(move || {
                loop {
                    // This timeout is necessary to avoid holding the event lock
                    // that crossterm::event::read() acquires. In particular,
                    // reading the cursor position (crossterm::cursor::position())
                    // needs to acquire the event lock, and so will fail if it
                    // can't acquire it within 2 sec. Resizing the terminal
                    // crashes the app if the cursor position can't be read.
                    if let Ok(true) = crossterm::event::poll(Duration::from_millis(100)) {
                        if let Ok(event) = crossterm::event::read() {
                            match event {
                                crossterm::event::Event::Key(key_event) => {
                                    app_event_tx.send(AppEvent::KeyEvent(key_event));
                                }
                                crossterm::event::Event::Resize(_, _) => {
                                    app_event_tx.send(AppEvent::RequestRedraw);
                                }
                                crossterm::event::Event::Paste(pasted) => {
                                    // Many terminals convert newlines to \r when pasting (e.g., iTerm2),
                                    // but tui-textarea expects \n. Normalize CR to LF.
                                    // [tui-textarea]: https://github.com/rhysd/tui-textarea/blob/4d18622eeac13b309e0ff6a55a46ac6706da68cf/src/textarea.rs#L782-L783
                                    // [iTerm2]: https://github.com/gnachman/iTerm2/blob/5d0c0d9f68523cbd0494dad5422998964a2ecd8d/sources/iTermPasteHelper.m#L206-L216
                                    let pasted = pasted.replace("\r", "\n");
                                    app_event_tx.send(AppEvent::Paste(pasted));
                                }
                                _ => {
                                    // Ignore any other events.
                                }
                            }
                        }
                    } else {
                        // Timeout expired, no `Event` is available
                    }
                }
            });
        }

        let login_status = get_login_status(&config);
        let should_show_onboarding =
            should_show_onboarding(login_status, &config, show_trust_screen);
        let app_state = if should_show_onboarding {
            let show_login_screen = should_show_login_screen(login_status, &config);
            let chat_widget_args = ChatWidgetArgs {
                config: config.clone(),
                initial_prompt,
                initial_images,
                enhanced_keys_supported,
            };
            AppState::Onboarding {
                screen: OnboardingScreen::new(OnboardingScreenArgs {
                    event_tx: app_event_tx.clone(),
                    codex_home: config.codex_home.clone(),
                    cwd: config.cwd.clone(),
                    show_trust_screen,
                    show_login_screen,
                    chat_widget_args,
                    login_status,
                }),
            }
        } else {
            let chat_widget = ChatWidget::new(
                config.clone(),
                conversation_manager.clone(),
                app_event_tx.clone(),
                initial_prompt,
                initial_images,
                enhanced_keys_supported,
            );
            AppState::Chat {
                widget: Box::new(chat_widget),
            }
        };

        let file_search = FileSearchManager::new(config.cwd.clone(), app_event_tx.clone());

        // Spawn a single scheduler thread that coalesces both debounced redraw
        // requests and animation frame requests, and emits a single Redraw event
        // at the earliest requested time.
        let (frame_tx, frame_rx) = channel::<Instant>();
        {
            let app_event_tx = app_event_tx.clone();
            std::thread::spawn(move || {
                use std::sync::mpsc::RecvTimeoutError;
                let mut next_deadline: Option<Instant> = None;
                loop {
                    if next_deadline.is_none() {
                        match frame_rx.recv() {
                            Ok(deadline) => next_deadline = Some(deadline),
                            Err(_) => break,
                        }
                    }

                    #[expect(clippy::expect_used)]
                    let deadline = next_deadline.expect("deadline set");
                    let now = Instant::now();
                    let timeout = if deadline > now {
                        deadline - now
                    } else {
                        Duration::from_millis(0)
                    };

                    match frame_rx.recv_timeout(timeout) {
                        Ok(new_deadline) => {
                            next_deadline =
                                Some(next_deadline.map_or(new_deadline, |d| d.min(new_deadline)));
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            app_event_tx.send(AppEvent::Redraw);
                            next_deadline = None;
                        }
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            });
        }
        // Temporarily disabled: Initialize agent infrastructure
        // let registry = Arc::new(SubagentRegistry::new());
        // let orchestrator_config = OrchestratorConfig::default();
        // let ast_tools = Arc::new(ASTAgentTools::new());
        // 
        // // Create orchestrator
        // let orchestrator = Arc::new(AgentOrchestrator::new(
        //     registry,
        //     orchestrator_config,
        //     OperatingMode::Build, // Will be updated when mode changes
        // ));
        // 
        // // Get progress receiver
        // let progress_receiver = {
        //     let orch = orchestrator.clone();
        //     tokio::spawn(async move {
        //         orch.progress_receiver().await
        //     })
        // };
        // 
        // // For now, create a dummy receiver since the orchestrator method needs to be fixed
        // let (_, dummy_rx) = tokio::sync::mpsc::unbounded_channel::<ProgressUpdate>();
        // let progress_receiver = Arc::new(tokio::sync::Mutex::new(dummy_rx));
        
        // Create the app first, then start progress monitoring
        let app = Self {
            server: conversation_manager,
            app_event_tx: app_event_tx.clone(),
            pending_history_lines: Vec::new(),
            app_event_rx,
            app_state,
            config,
            file_search,
            enhanced_keys_supported,
            commit_anim_running: Arc::new(AtomicBool::new(false)),
            frame_schedule_tx: frame_tx,
            mode_manager: ModeManager::new(OperatingMode::Build), // Default to Build mode
            agent_panel: AgentPanel::new(),
            // Temporarily disabled:
            // orchestrator: orchestrator.clone(),
            // ast_tools,
            // progress_receiver,
        };
        
        // TODO: Start progress monitoring when real orchestrator is available
        // app.start_progress_monitoring();
        
        app
    }

    fn schedule_frame_in(&self, dur: Duration) {
        let _ = self.frame_schedule_tx.send(Instant::now() + dur);
    }

    pub(crate) fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        // Schedule the first render immediately.
        let _ = self.frame_schedule_tx.send(Instant::now());

        while let Ok(event) = self.app_event_rx.recv() {
            match event {
                AppEvent::InsertHistory(lines) => {
                    self.pending_history_lines.extend(lines);
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                }
                AppEvent::RequestRedraw => {
                    self.schedule_frame_in(REDRAW_DEBOUNCE);
                }
                AppEvent::ScheduleFrameIn(dur) => {
                    self.schedule_frame_in(dur);
                }
                AppEvent::Redraw => {
                    // Synchronized update is not available in crossterm 0.28.1
                    // Just draw the frame directly
                    self.draw_next_frame(terminal)?;
                }
                AppEvent::StartCommitAnimation => {
                    if self
                        .commit_anim_running
                        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                        .is_ok()
                    {
                        let tx = self.app_event_tx.clone();
                        let running = self.commit_anim_running.clone();
                        thread::spawn(move || {
                            while running.load(Ordering::Relaxed) {
                                thread::sleep(Duration::from_millis(50));
                                tx.send(AppEvent::CommitTick);
                            }
                        });
                    }
                }
                AppEvent::StopCommitAnimation => {
                    self.commit_anim_running.store(false, Ordering::Release);
                }
                AppEvent::CommitTick => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.on_commit_tick();
                    }
                }
                AppEvent::KeyEvent(key_event) => {
                    match key_event {
                        KeyEvent {
                            code: KeyCode::Char('c'),
                            modifiers: crossterm::event::KeyModifiers::CONTROL,
                            kind: KeyEventKind::Press,
                            ..
                        } => match &mut self.app_state {
                            AppState::Chat { widget } => {
                                widget.on_ctrl_c();
                            }
                            AppState::Onboarding { .. } => {
                                self.app_event_tx.send(AppEvent::ExitRequest);
                            }
                        },
                        KeyEvent {
                            code: KeyCode::BackTab,
                            kind: KeyEventKind::Press,
                            ..
                        } => {
                            // Shift+Tab: Cycle through operating modes
                            self.app_event_tx.send(AppEvent::CycleModes);
                        }
                        KeyEvent {
                            code: KeyCode::Char('s'),
                            modifiers: crossterm::event::KeyModifiers::CONTROL,
                            kind: KeyEventKind::Press,
                            ..
                        } => {
                            // Ctrl+S: Open save session dialog
                            self.app_event_tx.send(AppEvent::OpenSaveDialog);
                        }
                        KeyEvent {
                            code: KeyCode::Char('o'),
                            modifiers: crossterm::event::KeyModifiers::CONTROL,
                            kind: KeyEventKind::Press,
                            ..
                        } => {
                            // Ctrl+O: Open load session dialog
                            self.app_event_tx.send(AppEvent::OpenLoadDialog);
                        }
                        KeyEvent {
                            code: KeyCode::Char('j'),
                            modifiers: crossterm::event::KeyModifiers::CONTROL,
                            kind: KeyEventKind::Press,
                            ..
                        } => {
                            // Ctrl+J: Open message jump popup
                            self.app_event_tx.send(AppEvent::ShowMessageJump);
                        }
                        KeyEvent {
                            code: KeyCode::Char('a'),
                            modifiers: crossterm::event::KeyModifiers::CONTROL,
                            kind: KeyEventKind::Press,
                            ..
                        } => {
                            // Ctrl+A: Toggle agent panel
                            self.app_event_tx.send(AppEvent::ToggleAgentPanel);
                        }
                        KeyEvent {
                            code: KeyCode::Char('z'),
                            modifiers: crossterm::event::KeyModifiers::CONTROL,
                            kind: KeyEventKind::Press,
                            ..
                        } => {
                            #[cfg(unix)]
                            {
                                self.suspend(terminal)?;
                            }
                            // No-op on non-Unix platforms.
                        }
                        KeyEvent {
                            code: KeyCode::Char('d'),
                            modifiers: crossterm::event::KeyModifiers::CONTROL,
                            kind: KeyEventKind::Press,
                            ..
                        } => {
                            match &mut self.app_state {
                                AppState::Chat { widget } => {
                                    if widget.composer_is_empty() {
                                        self.app_event_tx.send(AppEvent::ExitRequest);
                                    } else {
                                        // Treat Ctrl+D as a normal key event when the composer
                                        // is not empty so that it doesn't quit the application
                                        // prematurely.
                                        self.dispatch_key_event(key_event);
                                    }
                                }
                                AppState::Onboarding { .. } => {
                                    self.app_event_tx.send(AppEvent::ExitRequest);
                                }
                            }
                        }
                        KeyEvent {
                            kind: KeyEventKind::Press | KeyEventKind::Repeat,
                            ..
                        } => {
                            self.dispatch_key_event(key_event);
                        }
                        _ => {
                            // Ignore Release key events.
                        }
                    };
                }
                AppEvent::Paste(text) => {
                    self.dispatch_paste_event(text);
                }
                AppEvent::CodexEvent(event) => {
                    self.dispatch_codex_event(event);
                }
                AppEvent::ExitRequest => {
                    break;
                }
                AppEvent::CodexOp(op) => match &mut self.app_state {
                    AppState::Chat { widget } => widget.submit_op(op),
                    AppState::Onboarding { .. } => {}
                },
                AppEvent::DiffResult(text) => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.add_diff_output(text);
                    }
                }
                AppEvent::DispatchCommand(command) => match command {
                    SlashCommand::New => {
                        // User accepted – switch to chat view.
                        let new_widget = Box::new(ChatWidget::new(
                            self.config.clone(),
                            self.server.clone(),
                            self.app_event_tx.clone(),
                            None,
                            Vec::new(),
                            self.enhanced_keys_supported,
                        ));
                        self.app_state = AppState::Chat { widget: new_widget };
                        self.app_event_tx.send(AppEvent::RequestRedraw);
                    }
                    SlashCommand::Init => {
                        // Guard: do not run if a task is active.
                        if let AppState::Chat { widget } = &mut self.app_state {
                            const INIT_PROMPT: &str = include_str!("../prompt_for_init_command.md");
                            widget.submit_text_message(INIT_PROMPT.to_string());
                        }
                    }
                    SlashCommand::Compact => {
                        if let AppState::Chat { widget } = &mut self.app_state {
                            widget.clear_token_usage();
                            self.app_event_tx.send(AppEvent::CodexOp(Op::Compact));
                        }
                    }
                    SlashCommand::Model => {
                        if let AppState::Chat { widget } = &mut self.app_state {
                            widget.open_model_popup();
                        }
                    }
                    SlashCommand::Approvals => {
                        if let AppState::Chat { widget } = &mut self.app_state {
                            widget.open_approvals_popup();
                        }
                    }
                    SlashCommand::Quit => {
                        break;
                    }
                    SlashCommand::Logout => {
                        if let Err(e) = agcodex_login::logout(&self.config.codex_home) {
                            tracing::error!("failed to logout: {e}");
                        }
                        break;
                    }
                    SlashCommand::Diff => {
                        if let AppState::Chat { widget } = &mut self.app_state {
                            widget.add_diff_in_progress();
                        }

                        let tx = self.app_event_tx.clone();
                        tokio::spawn(async move {
                            let text = match get_git_diff().await {
                                Ok((is_git_repo, diff_text)) => {
                                    if is_git_repo {
                                        diff_text
                                    } else {
                                        "`/diff` — _not inside a git repository_".to_string()
                                    }
                                }
                                Err(e) => format!("Failed to compute diff: {e}"),
                            };
                            tx.send(AppEvent::DiffResult(text));
                        });
                    }
                    SlashCommand::Mention => {
                        if let AppState::Chat { widget } = &mut self.app_state {
                            widget.insert_str("@");
                        }
                    }
                    SlashCommand::Status => {
                        if let AppState::Chat { widget } = &mut self.app_state {
                            widget.add_status_output();
                        }
                    }
                    SlashCommand::Mcp => {
                        if let AppState::Chat { widget } = &mut self.app_state {
                            widget.add_mcp_output();
                        }
                    }
                    #[cfg(debug_assertions)]
                    SlashCommand::TestApproval => {
                        use agcodex_core::protocol::EventMsg;
                        use std::collections::HashMap;

                        use agcodex_core::protocol::ApplyPatchApprovalRequestEvent;
                        use agcodex_core::protocol::FileChange;

                        self.app_event_tx.send(AppEvent::CodexEvent(Event {
                            id: "1".to_string(),
                            // msg: EventMsg::ExecApprovalRequest(ExecApprovalRequestEvent {
                            //     call_id: "1".to_string(),
                            //     command: vec!["git".into(), "apply".into()],
                            //     cwd: self.config.cwd.clone(),
                            //     reason: Some("test".to_string()),
                            // }),
                            msg: EventMsg::ApplyPatchApprovalRequest(
                                ApplyPatchApprovalRequestEvent {
                                    call_id: "1".to_string(),
                                    changes: HashMap::from([
                                        (
                                            PathBuf::from("/tmp/test.txt"),
                                            FileChange::Add {
                                                content: "test".to_string(),
                                            },
                                        ),
                                        (
                                            PathBuf::from("/tmp/test2.txt"),
                                            FileChange::Update {
                                                unified_diff: "+test\n-test2".to_string(),
                                                move_path: None,
                                            },
                                        ),
                                    ]),
                                    reason: None,
                                    grant_root: Some(PathBuf::from("/tmp")),
                                },
                            ),
                        }));
                    }
                },
                AppEvent::OnboardingAuthComplete(result) => {
                    if let AppState::Onboarding { screen } = &mut self.app_state {
                        screen.on_auth_complete(result);
                    }
                }
                AppEvent::OnboardingComplete(ChatWidgetArgs {
                    config,
                    enhanced_keys_supported,
                    initial_images,
                    initial_prompt,
                }) => {
                    self.app_state = AppState::Chat {
                        widget: Box::new(ChatWidget::new(
                            config,
                            self.server.clone(),
                            self.app_event_tx.clone(),
                            initial_prompt,
                            initial_images,
                            enhanced_keys_supported,
                        )),
                    }
                }
                AppEvent::StartFileSearch(query) => {
                    if !query.is_empty() {
                        self.file_search.on_user_query(query);
                    }
                }
                AppEvent::FileSearchResult { query, matches } => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.apply_file_search_result(query, matches);
                    }
                }
                AppEvent::UpdateReasoningEffort(effort) => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.set_reasoning_effort(effort);
                    }
                }
                AppEvent::UpdateModel(model) => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.set_model(model);
                    }
                }
                AppEvent::UpdateAskForApprovalPolicy(policy) => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.set_approval_policy(policy);
                    }
                }
                AppEvent::UpdateSandboxPolicy(policy) => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.set_sandbox_policy(policy);
                    }
                }
                AppEvent::CycleModes => {
                    let new_mode = self.mode_manager.cycle();
                    tracing::info!("Switched to {:?} mode", new_mode);
                    
                    // TODO: Update orchestrator operating mode when available
                    // In a real implementation, you'd want to recreate the orchestrator
                    // or have a method to update its mode.
                    tracing::debug!("Mode switched to {:?} - will apply to future agent executions", new_mode);
                    
                    // The ModeIndicator widget will display the new mode visually
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                }
                AppEvent::OpenSaveDialog => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.open_save_dialog();
                    }
                }
                AppEvent::SaveSession { name, description } => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.save_session(name, description);
                    }
                }
                AppEvent::CloseSaveDialog => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.close_save_dialog();
                    }
                }
                AppEvent::ShowMessageJump => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.show_message_jump();
                    }
                }
                AppEvent::HideMessageJump => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.hide_message_jump();
                    }
                }
                AppEvent::JumpToMessage(index) => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.jump_to_message(index);
                    }
                }
                AppEvent::MessageJumpSearch(query) => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.update_message_jump_search(query);
                    }
                }
                AppEvent::MessageJumpCycleFilter => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.cycle_message_jump_filter();
                    }
                }
                AppEvent::OpenLoadDialog => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.show_load_dialog();
                    }
                }
                AppEvent::CloseLoadDialog => {
                    // Dialog will auto-close, just request redraw
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                }
                AppEvent::StartLoadSessionList => {
                    // Start loading session list in background
                    let tx = self.app_event_tx.clone();
                    tokio::spawn(async move {
                        // TODO: Initialize SessionManager and load sessions
                        // For now, send an empty result
                        tx.send(AppEvent::LoadSessionListResult(Ok(vec![])));
                    });
                }
                AppEvent::LoadSessionListResult(result) => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.apply_load_session_list_result(result);
                    }
                }
                AppEvent::LoadSession(session_id) => {
                    // TODO: Load the session with the given ID
                    tracing::info!("Loading session: {}", session_id);
                    // For now, just close the dialog
                    self.app_event_tx.send(AppEvent::CloseLoadDialog);
                }
                AppEvent::LoadSessionResult(result) => {
                    match result {
                        Ok(session_id) => {
                            tracing::info!("Successfully loaded session: {}", session_id);
                        }
                        Err(error) => {
                            tracing::error!("Failed to load session: {}", error);
                        }
                    }
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                }
                AppEvent::UpdateLoadDialogQuery(query) => {
                    if let AppState::Chat { widget } = &mut self.app_state {
                        widget.apply_load_dialog_query_update(query);
                    }
                }
                // Session browser events (placeholder implementations)
                AppEvent::OpenSessionBrowser => {
                    // TODO: Implement session browser
                    tracing::debug!("OpenSessionBrowser event (not implemented)");
                }
                AppEvent::CloseSessionBrowser => {
                    // TODO: Implement session browser
                    tracing::debug!("CloseSessionBrowser event (not implemented)");
                }
                AppEvent::SessionBrowserNavigate(_) => {
                    // TODO: Implement session browser navigation
                    tracing::debug!("SessionBrowserNavigate event (not implemented)");
                }
                AppEvent::SessionBrowserFocusNext => {
                    // TODO: Implement session browser focus
                    tracing::debug!("SessionBrowserFocusNext event (not implemented)");
                }
                AppEvent::SessionBrowserFocusPrevious => {
                    // TODO: Implement session browser focus
                    tracing::debug!("SessionBrowserFocusPrevious event (not implemented)");
                }
                AppEvent::SessionBrowserToggleViewMode => {
                    // TODO: Implement session browser view mode
                    tracing::debug!("SessionBrowserToggleViewMode event (not implemented)");
                }
                AppEvent::SessionBrowserCycleSort => {
                    // TODO: Implement session browser sorting
                    tracing::debug!("SessionBrowserCycleSort event (not implemented)");
                }
                AppEvent::SessionBrowserUpdateSearch(_) => {
                    // TODO: Implement session browser search
                    tracing::debug!("SessionBrowserUpdateSearch event (not implemented)");
                }
                AppEvent::SessionBrowserExecuteAction => {
                    // TODO: Implement session browser action execution
                    tracing::debug!("SessionBrowserExecuteAction event (not implemented)");
                }
                AppEvent::SessionBrowserDeleteSession(_) => {
                    // TODO: Implement session deletion
                    tracing::debug!("SessionBrowserDeleteSession event (not implemented)");
                }
                AppEvent::SessionBrowserExportSession { id: _, format: _ } => {
                    // TODO: Implement session export
                    tracing::debug!("SessionBrowserExportSession event (not implemented)");
                }
                AppEvent::SessionBrowserRenameSession { id: _, new_name: _ } => {
                    // TODO: Implement session renaming
                    tracing::debug!("SessionBrowserRenameSession event (not implemented)");
                }
                AppEvent::SessionBrowserToggleFavorite(_) => {
                    // TODO: Implement favorite toggling
                    tracing::debug!("SessionBrowserToggleFavorite event (not implemented)");
                }
                AppEvent::SessionBrowserDuplicateSession(_) => {
                    // TODO: Implement session duplication
                    tracing::debug!("SessionBrowserDuplicateSession event (not implemented)");
                }
                AppEvent::SessionBrowserShowConfirmation(_) => {
                    // TODO: Implement confirmation dialog
                    tracing::debug!("SessionBrowserShowConfirmation event (not implemented)");
                }
                AppEvent::SessionBrowserConfirmAction(_) => {
                    // TODO: Implement action confirmation
                    tracing::debug!("SessionBrowserConfirmAction event (not implemented)");
                }
                AppEvent::SessionBrowserToggleFavoritesFilter => {
                    // TODO: Implement favorites filter
                    tracing::debug!("SessionBrowserToggleFavoritesFilter event (not implemented)");
                }
                
                // TODO: Implement remaining event handlers
                AppEvent::OpenSessionBrowser
                | AppEvent::CloseSessionBrowser
                | AppEvent::SessionBrowserNavigate(_)
                | AppEvent::SessionBrowserFocusNext
                | AppEvent::SessionBrowserFocusPrevious
                | AppEvent::SessionBrowserToggleExpand
                | AppEvent::SessionBrowserSelect
                | AppEvent::SessionBrowserDelete
                | AppEvent::SessionBrowserFilter(_)
                | AppEvent::SessionBrowserSort(_)
                | AppEvent::StartHistoryGet
                | AppEvent::HistoryGetResult(_)
                | AppEvent::StartJumpToMessage(_)
                | AppEvent::JumpToMessage(_)
                | AppEvent::StartUndo
                | AppEvent::UndoComplete
                | AppEvent::StartRedo
                | AppEvent::RedoComplete
                | AppEvent::StartFork
                | AppEvent::ForkComplete(_)
                | AppEvent::OpenLoadDialog
                | AppEvent::CloseLoadDialog
                | AppEvent::StartLoadSessionList
                | AppEvent::LoadSessionListResult(_)
                | AppEvent::LoadSession(_) => {
                    // These events are not yet implemented
                    // TODO: Add implementations as features are completed
                }
                
                // ===== Agent Events =====
                AppEvent::StartAgent(invocation_request) => {
                    self.handle_start_agent(invocation_request);
                }
                AppEvent::AgentProgress { agent_id, progress, message } => {
                    self.agent_panel.update_progress(agent_id, progress, message);
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                }
                AppEvent::AgentComplete { agent_id, execution } => {
                    self.agent_panel.complete_agent(agent_id, execution);
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                }
                AppEvent::AgentFailed { agent_id, error } => {
                    self.agent_panel.fail_agent(agent_id, error);
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                }
                AppEvent::CancelAgent(agent_id) => {
                    self.handle_cancel_agent(agent_id);
                }
                AppEvent::ToggleAgentPanel => {
                    self.agent_panel.toggle_visibility();
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                }
                AppEvent::AgentPanelNavigateUp => {
                    if self.agent_panel.is_visible() {
                        self.agent_panel.navigate_up();
                        self.app_event_tx.send(AppEvent::RequestRedraw);
                    }
                }
                AppEvent::AgentPanelNavigateDown => {
                    if self.agent_panel.is_visible() {
                        self.agent_panel.navigate_down();
                        self.app_event_tx.send(AppEvent::RequestRedraw);
                    }
                }
                AppEvent::AgentPanelCancel => {
                    if self.agent_panel.is_visible() {
                        if let Some(agent_id) = self.agent_panel.selected_agent_id() {
                            self.app_event_tx.send(AppEvent::CancelAgent(agent_id));
                        }
                    }
                }
                AppEvent::AgentOutputChunk { agent_id, chunk } => {
                    self.agent_panel.add_output_chunk(agent_id, chunk);
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                }
            }
        }
        terminal.clear()?;

        Ok(())
    }

    #[cfg(unix)]
    fn suspend(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        tui::restore()?;
        // SAFETY: Unix-only code path. We intentionally send SIGTSTP to the
        // current process group (pid 0) to trigger standard job-control
        // suspension semantics. This FFI does not involve any raw pointers,
        // is not called from a signal handler, and uses a constant signal.
        // Errors from kill are acceptable (e.g., if already stopped) — the
        // subsequent re-init path will still leave the terminal in a good state.
        // We considered `nix`, but didn't think it was worth pulling in for this one call.
        unsafe { libc::kill(0, libc::SIGTSTP) };
        *terminal = tui::init(&self.config)?;
        terminal.clear()?;
        self.app_event_tx.send(AppEvent::RequestRedraw);
        Ok(())
    }

    pub(crate) fn token_usage(&self) -> agcodex_core::protocol::TokenUsage {
        match &self.app_state {
            AppState::Chat { widget } => widget.token_usage().clone(),
            AppState::Onboarding { .. } => agcodex_core::protocol::TokenUsage::default(),
        }
    }
    
    /// Get the current operating mode
    fn current_mode(&self) -> OperatingMode {
        self.mode_manager.current_mode()
    }

    /// Handle starting a new agent execution with enhanced simulation
    /// TODO: Replace with real orchestrator once core compilation issues are fixed
    fn handle_start_agent(&mut self, invocation_request: agcodex_core::subagents::InvocationRequest) {
        use agcodex_core::subagents::{ExecutionPlan, SubagentExecution};
        
        tracing::info!("Starting enhanced agent execution: {:?}", invocation_request.execution_plan);
        
        // Clone required data for the async task
        let app_event_tx = self.app_event_tx.clone();
        
        // Parse the execution plan and spawn appropriate simulated agents
        match invocation_request.execution_plan {
            ExecutionPlan::Single(invocation) => {
                self.spawn_enhanced_agent(invocation, invocation_request.context);
            }
            ExecutionPlan::Sequential(chain) => {
                // Spawn them one by one with proper chaining
                for (i, invocation) in chain.agents.into_iter().enumerate() {
                    let context = if i == 0 {
                        invocation_request.context.clone()
                    } else {
                        format!("Chained from previous agent (step {})", i)
                    };
                    // Add delay for sequential execution
                    let delay = std::time::Duration::from_millis((i as u64) * 500);
                    self.spawn_enhanced_agent_with_delay(invocation, context, delay);
                }
            }
            ExecutionPlan::Parallel(invocations) => {
                for invocation in invocations {
                    self.spawn_enhanced_agent(invocation, invocation_request.context.clone());
                }
            }
            ExecutionPlan::Mixed(steps) => {
                // Handle mixed execution with proper coordination
                let mut step_delay = 0u64;
                for step in steps {
                    match step {
                        agcodex_core::subagents::ExecutionStep::Single(invocation) => {
                            let delay = std::time::Duration::from_millis(step_delay * 200);
                            self.spawn_enhanced_agent_with_delay(invocation, invocation_request.context.clone(), delay);
                            step_delay += 1;
                        }
                        agcodex_core::subagents::ExecutionStep::Parallel(invocations) => {
                            for invocation in invocations {
                                let delay = std::time::Duration::from_millis(step_delay * 200);
                                self.spawn_enhanced_agent_with_delay(invocation, invocation_request.context.clone(), delay);
                            }
                            step_delay += 1;
                        }
                        agcodex_core::subagents::ExecutionStep::Barrier => {
                            // Implement barrier by adding extra delay
                            step_delay += 5; // Extra delay for barrier
                            tracing::debug!("Barrier step: adding extra coordination delay");
                        }
                    }
                }
            }
        }
        
        // Show the agent panel when agents are started
        self.agent_panel.set_visible(true);
        self.app_event_tx.send(AppEvent::RequestRedraw);
    }
    
    /// Spawn an enhanced agent with realistic behavior
    fn spawn_enhanced_agent(
        &mut self, 
        invocation: agcodex_core::subagents::AgentInvocation, 
        context: String
    ) {
        self.spawn_enhanced_agent_with_delay(invocation, context, std::time::Duration::from_millis(0));
    }
    
    /// Spawn an enhanced agent with a specified delay
    fn spawn_enhanced_agent_with_delay(
        &mut self, 
        invocation: agcodex_core::subagents::AgentInvocation, 
        context: String,
        delay: std::time::Duration
    ) {
        use agcodex_core::subagents::SubagentExecution;
        
        let mut execution = SubagentExecution::new(invocation.agent_name.clone());
        execution.start();
        
        let agent_id = execution.id;
        let agent_name = invocation.agent_name.clone();
        let parameters = invocation.parameters.clone();
        
        // Add to the agent panel
        self.agent_panel.add_agent(execution);
        
        // Spawn enhanced agent execution in background
        let app_event_tx = self.app_event_tx.clone();
        
        tokio::spawn(async move {
            // Initial delay if specified
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            
            // Enhanced simulation based on agent type
            let agent_steps = Self::get_agent_steps(&agent_name);
            let total_steps = agent_steps.len();
            
            // Execute each step with realistic timing
            for (i, step) in agent_steps.iter().enumerate() {
                let progress = (i as f32 + 1.0) / total_steps as f32;
                
                app_event_tx.send(AppEvent::AgentProgress {
                    agent_id,
                    progress,
                    message: step.clone(),
                });
                
                // Realistic timing based on step complexity
                let step_duration = Self::get_step_duration(step);
                tokio::time::sleep(step_duration).await;
            }
            
            // Generate realistic output based on agent type
            let output = Self::generate_agent_output(&agent_name, &parameters, &context);
            let modified_files = Self::get_simulated_modified_files(&agent_name);
            
            let mut completed_execution = SubagentExecution::new(agent_name);
            completed_execution.complete(output, modified_files);
            
            app_event_tx.send(AppEvent::AgentComplete {
                agent_id,
                execution: completed_execution,
            });
        });
    }
    
    /// Get realistic steps for different agent types
    fn get_agent_steps(agent_name: &str) -> Vec<String> {
        match agent_name {
            "code-reviewer" => vec![
                "Initializing code review analysis...".to_string(),
                "Parsing AST and building symbol tables...".to_string(),
                "Analyzing code quality metrics...".to_string(),
                "Checking for security vulnerabilities...".to_string(),
                "Evaluating performance patterns...".to_string(),
                "Generating review findings...".to_string(),
                "Finalizing recommendations...".to_string(),
            ],
            "refactorer" => vec![
                "Analyzing code structure...".to_string(),
                "Identifying refactoring opportunities...".to_string(),
                "Calculating complexity metrics...".to_string(),
                "Planning structural improvements...".to_string(),
                "Generating refactoring suggestions...".to_string(),
                "Validating proposed changes...".to_string(),
            ],
            "debugger" => vec![
                "Scanning for potential bugs...".to_string(),
                "Analyzing control flow...".to_string(),
                "Checking error handling patterns...".to_string(),
                "Validating input sanitization...".to_string(),
                "Generating debug report...".to_string(),
            ],
            "test-writer" => vec![
                "Analyzing code coverage...".to_string(),
                "Identifying test gaps...".to_string(),
                "Generating test cases...".to_string(),
                "Creating mock objects...".to_string(),
                "Validating test quality...".to_string(),
            ],
            "performance" => vec![
                "Profiling execution paths...".to_string(),
                "Analyzing memory usage patterns...".to_string(),
                "Identifying bottlenecks...".to_string(),
                "Calculating algorithmic complexity...".to_string(),
                "Generating optimization recommendations...".to_string(),
            ],
            "security" => vec![
                "Scanning for OWASP Top 10 vulnerabilities...".to_string(),
                "Analyzing authentication flows...".to_string(),
                "Checking input validation...".to_string(),
                "Evaluating cryptographic usage...".to_string(),
                "Generating security assessment...".to_string(),
            ],
            "docs" => vec![
                "Analyzing code documentation...".to_string(),
                "Extracting API signatures...".to_string(),
                "Generating usage examples...".to_string(),
                "Creating documentation structure...".to_string(),
                "Finalizing documentation...".to_string(),
            ],
            _ => vec![
                "Initializing agent...".to_string(),
                "Analyzing codebase...".to_string(),
                "Processing requirements...".to_string(),
                "Generating results...".to_string(),
                "Finalizing output...".to_string(),
            ],
        }
    }
    
    /// Get realistic timing for different step types
    fn get_step_duration(step: &str) -> std::time::Duration {
        if step.contains("Initializing") {
            std::time::Duration::from_millis(800)
        } else if step.contains("Parsing") || step.contains("AST") {
            std::time::Duration::from_millis(1500)
        } else if step.contains("Analyzing") {
            std::time::Duration::from_millis(1200)
        } else if step.contains("Generating") {
            std::time::Duration::from_millis(1000)
        } else if step.contains("Finalizing") {
            std::time::Duration::from_millis(600)
        } else {
            std::time::Duration::from_millis(1000)
        }
    }
    
    /// Generate realistic output based on agent type
    fn generate_agent_output(
        agent_name: &str,
        parameters: &std::collections::HashMap<String, String>,
        context: &str
    ) -> String {
        let param_str = if parameters.is_empty() {
            "no parameters".to_string()
        } else {
            format!("{} parameters", parameters.len())
        };
        
        match agent_name {
            "code-reviewer" => format!(
                "# Code Review Report\n\n\
                **Summary**: Comprehensive code review completed\n\
                **Files Analyzed**: 23\n\
                **Issues Found**: 5 medium priority, 2 low priority\n\
                **Quality Score**: 87/100\n\n\
                ## Key Findings\n\
                - Function complexity could be reduced in 3 locations\n\
                - Missing error handling in async operations\n\
                - Opportunity for performance optimization in hot paths\n\n\
                **Context**: {}\n**Parameters**: {}", 
                context, param_str
            ),
            "refactorer" => format!(
                "# Refactoring Recommendations\n\n\
                **Analysis Complete**: Found 8 refactoring opportunities\n\
                **Estimated Impact**: 15% complexity reduction\n\
                **Risk Level**: Low\n\n\
                ## Suggested Changes\n\
                1. Extract common patterns into utility functions\n\
                2. Simplify conditional logic in core modules\n\
                3. Apply dependency injection pattern\n\n\
                **Context**: {}\n**Parameters**: {}",
                context, param_str
            ),
            "debugger" => format!(
                "# Debug Analysis Report\n\n\
                **Scan Complete**: No critical bugs detected\n\
                **Potential Issues**: 3 minor warnings\n\
                **Code Health**: Good\n\n\
                ## Analysis Results\n\
                - All error paths properly handled\n\
                - Memory management appears correct\n\
                - Consider adding more defensive checks\n\n\
                **Context**: {}\n**Parameters**: {}",
                context, param_str
            ),
            _ => format!(
                "# Agent Execution Report\n\n\
                **Agent**: {}\n\
                **Status**: Successfully completed\n\
                **Analysis**: Comprehensive codebase review performed\n\n\
                ## Results\n\
                - All requirements processed\n\
                - Recommendations generated\n\
                - Quality checks passed\n\n\
                **Context**: {}\n**Parameters**: {}",
                agent_name, context, param_str
            ),
        }
    }
    
    /// Get simulated modified files based on agent type
    fn get_simulated_modified_files(agent_name: &str) -> Vec<std::path::PathBuf> {
        match agent_name {
            "refactorer" => vec![
                std::path::PathBuf::from("src/core/refactored_module.rs"),
                std::path::PathBuf::from("src/utils/extracted_common.rs"),
            ],
            "test-writer" => vec![
                std::path::PathBuf::from("tests/integration_tests.rs"),
                std::path::PathBuf::from("tests/unit/new_test_suite.rs"),
            ],
            "docs" => vec![
                std::path::PathBuf::from("docs/api_reference.md"),
                std::path::PathBuf::from("README.md"),
            ],
            _ => vec![], // Most agents don't modify files directly
        }
    }
    
    /// Handle agent cancellation (enhanced simulation)
    fn handle_cancel_agent(&mut self, agent_id: Uuid) {
        tracing::info!("Cancelling agent with ID: {}", agent_id);
        
        // TODO: When real orchestrator is available, cancel in the orchestrator
        // self.orchestrator.cancel();
        
        // Update the agent panel
        self.agent_panel.cancel_agent(agent_id);
        self.app_event_tx.send(AppEvent::RequestRedraw);
        
        // For enhanced simulation, just log the cancellation
        tracing::debug!("Agent {} cancelled via enhanced simulation", agent_id);
    }

    /// Get the current operating mode
    pub(crate) const fn current_mode(&self) -> OperatingMode {
        self.mode_manager.current_mode()
    }

    fn draw_next_frame(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        if matches!(self.app_state, AppState::Onboarding { .. }) {
            terminal.clear()?;
        }

        let screen_size = terminal.size()?;
        let last_known_screen_size = terminal.last_known_screen_size;
        if screen_size != last_known_screen_size {
            let cursor_pos = terminal.get_cursor_position()?;
            let last_known_cursor_pos = terminal.last_known_cursor_pos;
            if cursor_pos.y != last_known_cursor_pos.y {
                // The terminal was resized. The only point of reference we have for where our viewport
                // was moved is the cursor position.
                // NB this assumes that the cursor was not wrapped as part of the resize.
                let cursor_delta = cursor_pos.y as i32 - last_known_cursor_pos.y as i32;

                let new_viewport_area = terminal.viewport_area.offset(Offset {
                    x: 0,
                    y: cursor_delta,
                });
                terminal.set_viewport_area(new_viewport_area);
                terminal.clear()?;
            }
        }

        let size = terminal.size()?;
        let desired_height = match &self.app_state {
            AppState::Chat { widget } => widget.desired_height(size.width),
            AppState::Onboarding { .. } => size.height,
        };

        let mut area = terminal.viewport_area;
        area.height = desired_height.min(size.height);
        area.width = size.width;
        if area.bottom() > size.height {
            terminal
                .backend_mut()
                .scroll_region_up(0..area.top(), area.bottom() - size.height)?;
            area.y = size.height - area.height;
        }
        if area != terminal.viewport_area {
            terminal.clear()?;
            terminal.set_viewport_area(area);
        }
        if !self.pending_history_lines.is_empty() {
            crate::insert_history::insert_history_lines(
                terminal,
                self.pending_history_lines.clone(),
            );
            self.pending_history_lines.clear();
        }
        // Extract the current mode before the mutable borrow
        let current_mode = self.current_mode();

        terminal.draw(|frame| match &mut self.app_state {
            AppState::Chat { widget } => {
                // Determine if agent panel is visible
                let agent_panel_visible = self.agent_panel.is_visible();
                
                // Create main layout
                let main_constraints = if agent_panel_visible {
                    vec![
                        Constraint::Length(3),  // Mode indicator height
                        Constraint::Min(0),     // Chat widget
                        Constraint::Length(15), // Agent panel height
                    ]
                } else {
                    vec![
                        Constraint::Length(3), // Mode indicator height
                        Constraint::Min(0),    // Chat widget takes the rest
                    ]
                };
                
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(main_constraints)
                    .split(frame.area());

                // Create horizontal layout for the top area (mode indicator on the right)
                let top_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Min(0),     // Empty space on the left
                        Constraint::Length(15), // Mode indicator width
                    ])
                    .split(main_layout[0]);

                // Render mode indicator
                let mode_indicator = ModeIndicator::new(current_mode);
                frame.render_widget(mode_indicator, top_layout[1]);

                // Render chat widget
                let chat_area = main_layout[1];
                if let Some((x, y)) = widget.cursor_pos(chat_area) {
                    frame.set_cursor_position((x, y));
                }
                frame.render_widget_ref(&**widget, chat_area);
                
                // Render agent panel if visible
                if agent_panel_visible && main_layout.len() > 2 {
                    frame.render_widget_ref(&self.agent_panel, main_layout[2]);
                }
            }
            AppState::Onboarding { screen } => {
                // For onboarding, still show mode indicator but simpler layout
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Mode indicator height
                        Constraint::Min(0),    // Onboarding screen takes the rest
                    ])
                    .split(frame.area());

                let top_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Min(0),     // Empty space on the left
                        Constraint::Length(15), // Mode indicator width
                    ])
                    .split(layout[0]);

                // Render mode indicator
                let mode_indicator = ModeIndicator::new(current_mode);
                frame.render_widget(mode_indicator, top_layout[1]);

                // Render onboarding screen
                frame.render_widget_ref(&*screen, layout[1]);
            }
        })?;
        Ok(())
    }

    /// Dispatch a KeyEvent to the current view and let it decide what to do
    /// with it.
    fn dispatch_key_event(&mut self, key_event: KeyEvent) {
        // Check if agent panel is visible and should handle the key event
        if self.agent_panel.is_visible() {
            match key_event {
                KeyEvent {
                    code: KeyCode::Up, 
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.app_event_tx.send(AppEvent::AgentPanelNavigateUp);
                    return;
                }
                KeyEvent {
                    code: KeyCode::Down,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.app_event_tx.send(AppEvent::AgentPanelNavigateDown);
                    return;
                }
                KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.app_event_tx.send(AppEvent::AgentPanelCancel);
                    return;
                }
                KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.agent_panel.set_visible(false);
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                    return;
                }
                KeyEvent {
                    code: KeyCode::Char('c'),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.agent_panel.clear_completed();
                    self.app_event_tx.send(AppEvent::RequestRedraw);
                    return;
                }
                _ => {
                    // Let other keys fall through to normal handling
                }
            }
        }
        
        match &mut self.app_state {
            AppState::Chat { widget } => {
                widget.handle_key_event(key_event);
            }
            AppState::Onboarding { screen } => match key_event.code {
                KeyCode::Char('q') => {
                    self.app_event_tx.send(AppEvent::ExitRequest);
                }
                _ => screen.handle_key_event(key_event),
            },
        }
    }

    fn dispatch_paste_event(&mut self, pasted: String) {
        match &mut self.app_state {
            AppState::Chat { widget } => widget.handle_paste(pasted),
            AppState::Onboarding { .. } => {}
        }
    }

    fn dispatch_codex_event(&mut self, event: Event) {
        match &mut self.app_state {
            AppState::Chat { widget } => widget.handle_codex_event(event),
            AppState::Onboarding { .. } => {}
        }
    }
}

fn should_show_onboarding(
    login_status: LoginStatus,
    config: &Config,
    show_trust_screen: bool,
) -> bool {
    if show_trust_screen {
        return true;
    }

    should_show_login_screen(login_status, config)
}

fn should_show_login_screen(login_status: LoginStatus, config: &Config) -> bool {
    // Only show the login screen for providers that actually require OpenAI auth
    // (OpenAI or equivalents). For OSS/other providers, skip login entirely.
    if !config.model_provider.requires_openai_auth {
        return false;
    }

    match login_status {
        LoginStatus::NotAuthenticated => true,
        LoginStatus::AuthMode(method) => method != config.preferred_auth_method,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agcodex_core::config::ConfigOverrides;
    use agcodex_core::config::ConfigToml;
    use agcodex_login::AuthMode;

    fn make_config(preferred: AuthMode) -> Config {
        let mut cfg = Config::load_from_base_config_with_overrides(
            ConfigToml::default(),
            ConfigOverrides::default(),
            std::env::temp_dir(),
        )
        .expect("load default config");
        cfg.preferred_auth_method = preferred;
        cfg
    }

    #[test]
    fn shows_login_when_not_authenticated() {
        let cfg = make_config(AuthMode::ChatGPT);
        assert!(should_show_login_screen(
            LoginStatus::NotAuthenticated,
            &cfg
        ));
    }

    #[test]
    fn shows_login_when_api_key_but_prefers_chatgpt() {
        let cfg = make_config(AuthMode::ChatGPT);
        assert!(should_show_login_screen(
            LoginStatus::AuthMode(AuthMode::ApiKey),
            &cfg
        ))
    }

    #[test]
    fn hides_login_when_api_key_and_prefers_api_key() {
        let cfg = make_config(AuthMode::ApiKey);
        assert!(!should_show_login_screen(
            LoginStatus::AuthMode(AuthMode::ApiKey),
            &cfg
        ))
    }

    #[test]
    fn hides_login_when_chatgpt_and_prefers_chatgpt() {
        let cfg = make_config(AuthMode::ChatGPT);
        assert!(!should_show_login_screen(
            LoginStatus::AuthMode(AuthMode::ChatGPT),
            &cfg
        ))
    }
}
