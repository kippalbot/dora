use dora_node_api::arrow::array::{AsArray, StringArray};
use dora_node_api::dora_core::config::{DataId, NodeId};
use dora_node_api::{DoraNode, Event, EventStream, Parameter};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

// TUI imports
use crossterm::{
    event::{self, Event as CrosstermEvent, KeyCode, KeyEventKind, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use std::io::stdout;

// Message types for communication between threads
#[derive(Debug, Clone)]
enum AppMessage {
    TextChunk {
        participant: String,
        content: String,
    },
    Complete {
        participant: String,
    },
    Status {
        participant: String,
        status: String,
    },
}

// Application state
#[derive(Debug, Clone)]
struct ParticipantState {
    messages: Vec<String>,
    current_message: String,
    status: String,
    scroll_offset: u16,
    auto_scroll: bool,
}

impl ParticipantState {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            current_message: String::new(),
            status: "idle".to_string(),
            scroll_offset: 0,
            auto_scroll: true,
        }
    }

    fn add_chunk(&mut self, content: &str) {
        self.current_message.push_str(content);
    }

    fn complete_message(&mut self) {
        if !self.current_message.is_empty() {
            self.messages.push(self.current_message.clone());
            self.current_message.clear();
        }
    }

    fn set_status(&mut self, status: &str) {
        self.status = status.to_string();
    }

    fn get_display_content(&self) -> String {
        let mut content = self.messages.join("\n\n");
        if !self.current_message.is_empty() {
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&self.current_message);
        }
        content
    }

    fn update_scroll(&mut self, content_height: u16, viewport_height: u16) {
        // Auto-scroll to bottom only if auto_scroll is enabled
        if self.auto_scroll {
            if content_height > viewport_height {
                self.scroll_offset = content_height.saturating_sub(viewport_height);
            } else {
                self.scroll_offset = 0;
            }
        }
    }

    fn scroll_up(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    fn scroll_down(&mut self, content_height: u16, viewport_height: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
        let max_scroll = content_height.saturating_sub(viewport_height);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
        // Re-enable auto-scroll if at bottom
        if self.scroll_offset >= max_scroll {
            self.auto_scroll = true;
        }
    }

    fn page_up(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
    }

    fn page_down(&mut self, content_height: u16, viewport_height: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(10);
        let max_scroll = content_height.saturating_sub(viewport_height);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }
        // Re-enable auto-scroll if at bottom
        if self.scroll_offset >= max_scroll {
            self.auto_scroll = true;
        }
    }
}

struct AppState {
    participants: HashMap<String, ParticipantState>,
    input_buffer: String,
    focused_panel: String,
    reset_button_area: Option<Rect>,
}

impl AppState {
    fn new() -> Self {
        let mut participants = HashMap::new();
        participants.insert("llm1".to_string(), ParticipantState::new());
        participants.insert("llm2".to_string(), ParticipantState::new());
        participants.insert("judge".to_string(), ParticipantState::new());

        Self {
            participants,
            input_buffer: String::new(),
            focused_panel: "judge".to_string(),
            reset_button_area: None,
        }
    }

    fn cycle_focus(&mut self) {
        self.focused_panel = match self.focused_panel.as_str() {
            "judge" => "llm1".to_string(),
            "llm1" => "llm2".to_string(),
            "llm2" => "judge".to_string(),
            _ => "judge".to_string(),
        };
    }

    fn handle_message(&mut self, message: AppMessage) {
        match message {
            AppMessage::TextChunk {
                participant,
                content,
            } => {
                if let Some(state) = self.participants.get_mut(&participant) {
                    state.add_chunk(&content);
                    // Auto-scroll to show new content (estimate 100 lines viewport)
                    self.update_scroll_for_participant(&participant);
                }
            }
            AppMessage::Complete { participant } => {
                if let Some(state) = self.participants.get_mut(&participant) {
                    state.complete_message();
                    state.set_status("complete");
                    self.update_scroll_for_participant(&participant);
                }
            }
            AppMessage::Status {
                participant,
                status,
            } => {
                if let Some(state) = self.participants.get_mut(&participant) {
                    state.set_status(&status);
                }
            }
        }
    }

    fn update_scroll_for_participant(&mut self, participant: &str) {
        // Mark that we need to update scroll on next render
        // Actual scroll update happens during render when we know viewport size
        if let Some(state) = self.participants.get_mut(participant) {
            // Just mark content as changed, scroll will update during render
            let _ = state.get_display_content();
        }
    }

    fn update_scroll_in_render(
        &mut self,
        participant: &str,
        content_height: u16,
        viewport_height: u16,
    ) {
        if let Some(state) = self.participants.get_mut(participant) {
            state.update_scroll(content_height, viewport_height);
        }
    }

    fn get_participant_content(&self, participant: &str) -> String {
        self.participants
            .get(participant)
            .map(|state| state.get_display_content())
            .unwrap_or_default()
    }

    fn get_participant_status(&self, participant: &str) -> String {
        self.participants
            .get(participant)
            .map(|state| state.status.clone())
            .unwrap_or("unknown".to_string())
    }

    fn clear_input(&mut self) {
        self.input_buffer.clear();
    }

    fn push_input_char(&mut self, ch: char) {
        self.input_buffer.push(ch);
    }

    fn get_input_buffer(&self) -> &str {
        &self.input_buffer
    }
}

// Control message sender
fn send_control_message(
    node: &mut DoraNode,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if message.trim().to_lowercase() == "reset" {
        // Send reset command to bridges
        let payload = r#"{"command": "reset"}"#;
        node.send_output(
            DataId::from("bridge_control".to_string()),
            Default::default(),
            StringArray::from(vec![payload]),
        )?;
    } else {
        // Send prompt as JSON to judge control input
        let payload = format!(r#"{{"prompt": "{}"}}"#, message);
        node.send_output(
            DataId::from("control".to_string()),
            Default::default(),
            StringArray::from(vec![payload.as_str()]),
        )?;
    }
    Ok(())
}

// TUI Application
struct App {
    state: AppState,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            state: AppState::new(),
            should_quit: false,
        }
    }

    fn handle_message(&mut self, message: AppMessage) {
        self.state.handle_message(message);
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Option<String> {
        match key.code {
            KeyCode::Char('q') if key.kind == KeyEventKind::Press => {
                self.should_quit = true;
                None
            }
            KeyCode::Char('r')
                if key.kind == KeyEventKind::Press
                    && key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                // Ctrl+R to reset bridges
                Some("reset".to_string())
            }
            KeyCode::Tab if key.kind == KeyEventKind::Press => {
                self.state.cycle_focus();
                None
            }
            KeyCode::Up if key.kind == KeyEventKind::Press => {
                let focused = self.state.focused_panel.clone();
                if let Some(state) = self.state.participants.get_mut(&focused) {
                    state.scroll_up();
                }
                None
            }
            KeyCode::Down if key.kind == KeyEventKind::Press => {
                let focused = self.state.focused_panel.clone();
                let content = self.state.get_participant_content(&focused);
                let content_height = content.lines().count() as u16;
                // Estimate viewport height - will be accurate after next render
                let viewport_height = match focused.as_str() {
                    "judge" => 20, // Judge gets ~40% of space
                    _ => 30,       // LLM1/LLM2 get ~30% each
                };
                if let Some(state) = self.state.participants.get_mut(&focused) {
                    state.scroll_down(content_height, viewport_height);
                }
                None
            }
            KeyCode::PageUp if key.kind == KeyEventKind::Press => {
                let focused = self.state.focused_panel.clone();
                if let Some(state) = self.state.participants.get_mut(&focused) {
                    state.page_up();
                }
                None
            }
            KeyCode::PageDown if key.kind == KeyEventKind::Press => {
                let focused = self.state.focused_panel.clone();
                let content = self.state.get_participant_content(&focused);
                let content_height = content.lines().count() as u16;
                // Estimate viewport height - will be accurate after next render
                let viewport_height = match focused.as_str() {
                    "judge" => 20, // Judge gets ~40% of space
                    _ => 30,       // LLM1/LLM2 get ~30% each
                };
                if let Some(state) = self.state.participants.get_mut(&focused) {
                    state.page_down(content_height, viewport_height);
                }
                None
            }
            KeyCode::Char(c) if key.kind == KeyEventKind::Press => {
                self.state.push_input_char(c);
                None
            }
            KeyCode::Backspace if key.kind == KeyEventKind::Press => {
                self.state.input_buffer.pop();
                None
            }
            KeyCode::Enter if key.kind == KeyEventKind::Press => {
                // Message will be sent by main loop
                None
            }
            KeyCode::Esc if key.kind == KeyEventKind::Press => {
                self.state.clear_input();
                None
            }
            _ => None,
        }
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(1),
            Constraint::Length(3),
        ]);
        let [header_area, main_area, status_area, input_area] = layout.areas(area);

        // Header
        let header = Paragraph::new("LLM Debate Monitor")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        header.render(header_area, buf);

        // Main content area
        let main_layout = Layout::vertical([Constraint::Ratio(2, 5), Constraint::Ratio(3, 5)]);
        let [judge_area, debaters_area] = main_layout.areas(main_area);

        // Debaters panels (bottom 60%)
        let debaters_layout =
            Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]);
        let [llm1_area, llm2_area] = debaters_layout.areas(debaters_area);

        // Update scroll positions for all panels before rendering
        self.update_panel_scroll("judge", judge_area);
        self.update_panel_scroll("llm1", llm1_area);
        self.update_panel_scroll("llm2", llm2_area);

        // Judge panel (top 40%)
        self.render_participant_panel(
            "judge",
            "Judge (Moderator)",
            Color::Magenta,
            judge_area,
            buf,
        );
        self.render_participant_panel("llm1", "LLM1 (Debater A)", Color::Cyan, llm1_area, buf);
        self.render_participant_panel("llm2", "LLM2 (Debater B)", Color::Green, llm2_area, buf);

        // Status bar
        self.render_status_bar(status_area, buf);

        // Input area
        let input_layout = Layout::horizontal([Constraint::Min(10), Constraint::Length(20)]);
        let [input_field_area, button_area] = input_layout.areas(input_area);

        let input = Paragraph::new(self.state.get_input_buffer())
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::bordered().title("Send prompt to Judge"));
        input.render(input_field_area, buf);

        let reset_button = Paragraph::new("Reset Bridges")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .block(Block::bordered());
        reset_button.render(button_area, buf);

        // Store button area for click detection
        self.state.reset_button_area = Some(button_area);
    }
}

impl App {
    fn update_panel_scroll(&mut self, participant_id: &str, area: Rect) {
        let block_inner = Block::bordered().inner(area);
        let viewport_height = block_inner.height;
        let viewport_width = block_inner.width;

        let content = self.state.get_participant_content(participant_id);

        // Calculate actual content height considering word wrapping
        let wrapped_lines: Vec<String> = content
            .lines()
            .flat_map(|line| {
                if line.is_empty() {
                    vec![String::new()]
                } else {
                    // Estimate wrapped lines based on viewport width
                    let line_len = line.len() as u16;
                    let num_wrapped = (line_len + viewport_width - 1) / viewport_width.max(1);
                    vec![line.to_string(); num_wrapped.max(1) as usize]
                }
            })
            .collect();

        let content_height = wrapped_lines.len() as u16;

        self.state
            .update_scroll_in_render(participant_id, content_height, viewport_height);
    }

    fn render_participant_panel(
        &self,
        participant_id: &str,
        title: &str,
        color: Color,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let content = self.state.get_participant_content(participant_id);
        let status = self.state.get_participant_status(participant_id);
        let is_focused = self.state.focused_panel == participant_id;

        // Highlight entire panel background if focused - use a subtle gray
        let bg_color = if is_focused {
            Some(Color::Rgb(60, 60, 70)) // Medium dark gray with blue tint
        } else {
            None
        };

        let border_style = if is_focused {
            Style::default()
                .fg(color)
                .add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };

        let mut block = Block::bordered()
            .title(format!("{} [{}]", title, status))
            .border_style(border_style);

        if let Some(bg) = bg_color {
            block = block.style(Style::default().bg(bg));
        }

        let inner_area = block.inner(area);
        block.render(area, buf);

        if !content.is_empty() {
            // Get scroll offset from state
            let scroll_offset = self
                .state
                .participants
                .get(participant_id)
                .map(|s| s.scroll_offset)
                .unwrap_or(0);

            // Calculate actual content height considering word wrapping
            let viewport_width = inner_area.width;
            let wrapped_lines: Vec<String> = content
                .lines()
                .flat_map(|line| {
                    if line.is_empty() {
                        vec![String::new()]
                    } else {
                        // Estimate wrapped lines based on viewport width
                        let line_len = line.len() as u16;
                        let num_wrapped = (line_len + viewport_width - 1) / viewport_width.max(1);
                        vec![line.to_string(); num_wrapped.max(1) as usize]
                    }
                })
                .collect();

            let content_height = wrapped_lines.len() as u16;
            let viewport_height = inner_area.height;

            let mut text_style = Style::default().fg(color);
            if let Some(bg) = bg_color {
                text_style = text_style.bg(bg);
            }

            let paragraph = Paragraph::new(content)
                .style(text_style)
                .wrap(Wrap { trim: true })
                .scroll((scroll_offset, 0));
            paragraph.render(inner_area, buf);

            // Render scrollbar if content is longer than viewport
            if content_height > viewport_height {
                let scrollbar = Scrollbar::default()
                    .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"))
                    .track_symbol(Some("│"))
                    .thumb_symbol("█");

                let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(
                    content_height.saturating_sub(viewport_height) as usize,
                )
                .position(scroll_offset as usize);

                scrollbar.render(inner_area, buf, &mut scrollbar_state);
            }
        }
    }

    fn render_status_bar(&self, area: Rect, buf: &mut Buffer) {
        let status_text = format!(
            "Tab: Switch | ↑/↓: Scroll | PgUp/PgDn: Page | Ctrl+R: Reset | q: Quit | Focused: {}",
            match self.state.focused_panel.as_str() {
                "judge" => "Judge",
                "llm1" => "LLM1",
                "llm2" => "LLM2",
                _ => "Unknown",
            }
        );

        let paragraph = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        paragraph.render(area, buf);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Dora node as a dynamic node
    let (mut node, mut events) = match DoraNode::init_from_node_id(NodeId::from(
        "debate-monitor".to_string(),
    )) {
        Ok(result) => result,
        Err(_e) => {
            // Only print error if we can't initialize terminal (console mode fallback)
            eprintln!("Failed to initialize Dora node");
            eprintln!(
                "Make sure the dataflow is running and contains a node with id 'debate-monitor'"
            );
            return Ok(());
        }
    };

    // Try to initialize terminal - if this fails, fall back to simple console output
    let use_tui = match enable_raw_mode() {
        Ok(()) => {
            // Enable mouse capture
            let _ = execute!(stdout(), crossterm::event::EnableMouseCapture);

            match Terminal::new(CrosstermBackend::new(stdout())) {
                Ok(mut terminal) => {
                    if terminal.clear().is_ok() {
                        Some(terminal)
                    } else {
                        let _ = execute!(stdout(), crossterm::event::DisableMouseCapture);
                        let _ = disable_raw_mode();
                        None
                    }
                }
                Err(_) => {
                    let _ = execute!(stdout(), crossterm::event::DisableMouseCapture);
                    let _ = disable_raw_mode();
                    None
                }
            }
        }
        Err(_) => None,
    };

    // Create app
    let mut app = App::new();

    match use_tui {
        Some(mut terminal) => {
            // TUI mode - no console output
            run_tui_mode(&mut terminal, &mut app, &mut events, &mut node)?;
        }
        None => {
            // Console mode - show startup messages
            println!("=== Console Mode Active ===");
            println!("Controls:");
            println!("  - Type prompts and press Enter to send to judge");
            println!("  - Press Ctrl+C to exit");
            println!("");
            run_console_mode(&mut app, &mut events, &mut node)?;
        }
    }

    Ok(())
}

fn run_tui_mode<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    events: &mut EventStream,
    node: &mut DoraNode,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Process Dora events with timeout to keep UI responsive
        // Note: EventStream::recv() is blocking, so we just check once per loop iteration
        if let Some(event) = events.recv_timeout(Duration::from_millis(5)) {
            match event {
                Event::Input { id, data, metadata } => {
                    let input_id = id.to_string();
                    let parameters = metadata.parameters;

                    // Extract text from the data
                    let text_array = data.as_string::<i32>();
                    let text = text_array
                        .iter()
                        .filter_map(|value| value.map(str::to_string))
                        .collect::<Vec<String>>()
                        .join(" ");

                    // Determine participant
                    let participant = if input_id.starts_with("llm1_") {
                        "llm1"
                    } else if input_id.starts_with("llm2_") {
                        "llm2"
                    } else if input_id.starts_with("judge_") {
                        "judge"
                    } else if input_id == "bundle_text" {
                        "judge"
                    } else {
                        continue;
                    };

                    // Handle different input types
                    if input_id.ends_with("_text") || input_id == "bundle_text" {
                        // Handle text input
                        let is_complete = match parameters.get("is_complete") {
                            Some(Parameter::Bool(b)) => *b,
                            Some(Parameter::String(s)) => s.to_lowercase() == "true",
                            _ => false,
                        };

                        let status_ended = match parameters.get("session_status") {
                            Some(Parameter::String(s)) => s.to_lowercase() == "ended",
                            _ => false,
                        };

                        // Send text chunk
                        app.handle_message(AppMessage::TextChunk {
                            participant: participant.to_string(),
                            content: text.clone(),
                        });

                        // Check if complete
                        if status_ended || is_complete {
                            app.handle_message(AppMessage::Complete {
                                participant: participant.to_string(),
                            });
                            app.handle_message(AppMessage::Status {
                                participant: participant.to_string(),
                                status: "complete".to_string(),
                            });
                        } else {
                            app.handle_message(AppMessage::Status {
                                participant: participant.to_string(),
                                status: "streaming".to_string(),
                            });
                        }
                    } else if input_id.ends_with("_status") {
                        // Handle status input
                        if !text.is_empty() {
                            app.handle_message(AppMessage::Status {
                                participant: participant.to_string(),
                                status: text,
                            });
                        }
                    }
                }
                Event::Stop(_) => {
                    break;
                }
                _ => {}
            }
        }

        // Render UI
        terminal.draw(|frame| {
            frame.render_widget(&mut *app, frame.area());
        })?;

        // Handle input
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                CrosstermEvent::Key(key) => {
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                        // Send control message
                        let input_text = app.state.get_input_buffer().to_string();
                        if !input_text.trim().is_empty() {
                            match send_control_message(node, &input_text) {
                                Ok(_) => {
                                    app.state.clear_input();
                                }
                                Err(_e) => {
                                    // Log error but don't crash - TUI might corrupt if we print
                                    // Just clear input to indicate something happened
                                    app.state.clear_input();
                                }
                            }
                        }
                    } else {
                        // Handle other key events - may return a command to send
                        if let Some(command) = app.handle_key_event(key) {
                            // Send the command (e.g., "reset")
                            let _ = send_control_message(node, &command);
                        }
                    }
                }
                CrosstermEvent::Mouse(mouse_event) => {
                    // Handle mouse clicks on reset button
                    if mouse_event.kind == MouseEventKind::Down(MouseButton::Left) {
                        if let Some(button_area) = app.state.reset_button_area {
                            // Check if click is within button area
                            let x = mouse_event.column;
                            let y = mouse_event.row;
                            if x >= button_area.x
                                && x < button_area.x + button_area.width
                                && y >= button_area.y
                                && y < button_area.y + button_area.height
                            {
                                // Send reset command
                                let _ = send_control_message(node, "reset");
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }

        // Small delay to prevent busy waiting
        thread::sleep(Duration::from_millis(10));
    }

    disable_raw_mode()?;
    Ok(())
}

fn run_console_mode(
    app: &mut App,
    events: &mut EventStream,
    _node: &mut DoraNode,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Process Dora events with timeout
        if let Some(event) = events.recv_timeout(Duration::from_millis(100)) {
            match event {
                Event::Input { id, data, metadata } => {
                    let input_id = id.to_string();
                    let parameters = metadata.parameters;

                    // Extract text
                    let text_array = data.as_string::<i32>();
                    let text = text_array
                        .iter()
                        .filter_map(|value| value.map(str::to_string))
                        .collect::<Vec<String>>()
                        .join(" ");

                    // Determine participant
                    let participant = if input_id.starts_with("llm1_") {
                        "llm1"
                    } else if input_id.starts_with("llm2_") {
                        "llm2"
                    } else if input_id.starts_with("judge_") {
                        "judge"
                    } else if input_id == "bundle_text" {
                        "judge"
                    } else {
                        continue;
                    };

                    // Display and process
                    if input_id.ends_with("_text") || input_id == "bundle_text" {
                        println!("[{}] {}", participant, text);

                        let is_complete = match parameters.get("is_complete") {
                            Some(Parameter::Bool(b)) => *b,
                            Some(Parameter::String(s)) => s.to_lowercase() == "true",
                            _ => false,
                        };

                        let status_ended = match parameters.get("session_status") {
                            Some(Parameter::String(s)) => s.to_lowercase() == "ended",
                            _ => false,
                        };

                        if status_ended || is_complete {
                            println!("[{}] ✓ Message complete", participant);
                        }
                    } else if input_id.ends_with("_status") {
                        println!("[{}] Status: {}", participant, text);
                    }
                }
                Event::Stop(_) => {
                    break;
                }
                _ => {}
            }
        }

        thread::sleep(Duration::from_millis(100));

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
