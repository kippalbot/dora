//! Dora Integration for MoFA FM
//!
//! Manages the lifecycle of dora bridges and routes data between
//! the dora dataflow and MoFA widgets.

use crossbeam_channel::{Receiver, Sender, bounded};
use mofa_dora_bridge::{
    controller::DataflowController,
    data::{AudioData, ChatMessage, LogEntry},
    dispatcher::DynamicNodeDispatcher,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use parking_lot::RwLock;

// NOTE: ParticipantAudioData removed - LED visualization is calculated in screen.rs
// from output waveform (more accurate since it reflects what's actually being played)

/// State shared between the UI and dora bridges
#[derive(Debug, Default)]
pub struct DoraState {
    /// Whether the dataflow is running
    pub dataflow_running: bool,
    /// Current dataflow ID
    pub dataflow_id: Option<String>,
    /// Connection states for each bridge
    pub audio_player_connected: bool,
    pub system_log_connected: bool,
    pub prompt_input_connected: bool,
    /// Audio buffer fill percentage
    pub buffer_fill: f64,
    /// Last received chat messages
    pub pending_chat_messages: Vec<ChatMessage>,
    /// Last received log entries
    pub pending_log_entries: Vec<LogEntry>,
}

/// Commands sent from UI to dora integration
#[derive(Debug, Clone)]
pub enum DoraCommand {
    /// Start the dataflow with optional environment variables
    StartDataflow {
        dataflow_path: PathBuf,
        env_vars: std::collections::HashMap<String, String>,
    },
    /// Stop the dataflow gracefully (default 15s grace period)
    StopDataflow,
    /// Stop the dataflow with custom grace duration (in seconds)
    StopDataflowWithGrace { grace_seconds: u64 },
    /// Force stop the dataflow immediately (0s grace period)
    ForceStopDataflow,
    /// Send a prompt to LLM
    SendPrompt { message: String },
    /// Send a control command
    SendControl { command: String },
    /// Update buffer status
    UpdateBufferStatus { fill_percentage: f64 },
}

/// Events sent from dora integration to UI
#[derive(Debug, Clone)]
pub enum DoraEvent {
    /// Dataflow started
    DataflowStarted { dataflow_id: String },
    /// Dataflow stopped
    DataflowStopped,
    /// Bridge connected
    BridgeConnected { bridge_name: String },
    /// Bridge disconnected
    BridgeDisconnected { bridge_name: String },
    /// Audio received
    AudioReceived { data: AudioData },
    // NOTE: ParticipantAudioReceived removed - LED visualization calculated in screen.rs from output waveform
    /// Chat message received
    ChatReceived { message: ChatMessage },
    /// Log entry received
    LogReceived { entry: LogEntry },
    /// Error occurred
    Error { message: String },
}

/// Dora integration manager
pub struct DoraIntegration {
    /// Shared state
    state: Arc<RwLock<DoraState>>,
    /// Command sender (UI -> dora thread)
    command_tx: Sender<DoraCommand>,
    /// Event receiver (dora thread -> UI)
    event_rx: Receiver<DoraEvent>,
    /// Worker thread handle
    worker_handle: Option<thread::JoinHandle<()>>,
    /// Stop signal
    stop_tx: Option<Sender<()>>,
}

impl DoraIntegration {
    /// Create a new dora integration (not started)
    pub fn new() -> Self {
        let (command_tx, command_rx) = bounded(100);
        let (event_tx, event_rx) = bounded(100);
        let (stop_tx, stop_rx) = bounded(1);

        let state = Arc::new(RwLock::new(DoraState::default()));
        let state_clone = Arc::clone(&state);

        // Spawn worker thread
        let handle = thread::spawn(move || {
            Self::run_worker(state_clone, command_rx, event_tx, stop_rx);
        });

        Self {
            state,
            command_tx,
            event_rx,
            worker_handle: Some(handle),
            stop_tx: Some(stop_tx),
        }
    }

    /// Get shared state reference
    pub fn state(&self) -> &Arc<RwLock<DoraState>> {
        &self.state
    }

    /// Send a command to the dora integration
    pub fn send_command(&self, cmd: DoraCommand) -> bool {
        self.command_tx.send(cmd).is_ok()
    }

    /// Start a dataflow with optional environment variables
    pub fn start_dataflow(&self, dataflow_path: impl Into<PathBuf>) -> bool {
        self.start_dataflow_with_env(dataflow_path, std::collections::HashMap::new())
    }

    /// Start a dataflow with environment variables
    pub fn start_dataflow_with_env(
        &self,
        dataflow_path: impl Into<PathBuf>,
        env_vars: std::collections::HashMap<String, String>,
    ) -> bool {
        self.send_command(DoraCommand::StartDataflow {
            dataflow_path: dataflow_path.into(),
            env_vars,
        })
    }

    /// Stop the current dataflow gracefully (default 15s grace period)
    pub fn stop_dataflow(&self) -> bool {
        self.send_command(DoraCommand::StopDataflow)
    }

    /// Stop the dataflow with a custom grace duration
    ///
    /// After the grace duration, nodes that haven't stopped will be killed (SIGKILL).
    pub fn stop_dataflow_with_grace(&self, grace_seconds: u64) -> bool {
        self.send_command(DoraCommand::StopDataflowWithGrace { grace_seconds })
    }

    /// Force stop the dataflow immediately (0s grace period)
    ///
    /// This will immediately kill all nodes without waiting for graceful shutdown.
    pub fn force_stop_dataflow(&self) -> bool {
        self.send_command(DoraCommand::ForceStopDataflow)
    }

    /// Send a prompt to LLM
    pub fn send_prompt(&self, message: impl Into<String>) -> bool {
        self.send_command(DoraCommand::SendPrompt {
            message: message.into(),
        })
    }

    /// Send a control command (e.g., "reset", "cancel")
    pub fn send_control(&self, command: impl Into<String>) -> bool {
        self.send_command(DoraCommand::SendControl {
            command: command.into(),
        })
    }

    /// Poll for events (non-blocking)
    pub fn poll_events(&self) -> Vec<DoraEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// Check if dataflow is running
    pub fn is_running(&self) -> bool {
        self.state.read().dataflow_running
    }

    /// Worker thread main loop
    fn run_worker(
        state: Arc<RwLock<DoraState>>,
        command_rx: Receiver<DoraCommand>,
        event_tx: Sender<DoraEvent>,
        stop_rx: Receiver<()>,
    ) {
        log::info!("Dora integration worker started");

        let mut dispatcher: Option<DynamicNodeDispatcher> = None;
        let mut last_status_check = std::time::Instant::now();
        let status_check_interval = std::time::Duration::from_secs(2);
        let mut dataflow_start_time: Option<std::time::Instant> = None;
        let startup_grace_period = std::time::Duration::from_secs(10); // Don't check status during startup

        loop {
            // Check for stop signal
            if stop_rx.try_recv().is_ok() {
                log::info!("Dora integration worker received stop signal");
                break;
            }

            // Process commands
            while let Ok(cmd) = command_rx.try_recv() {
                match cmd {
                    DoraCommand::StartDataflow { dataflow_path, env_vars } => {
                        log::info!("Starting dataflow: {:?}", dataflow_path);

                        // Set environment variables in both process env and controller
                        for (key, value) in &env_vars {
                            log::info!("Setting env var: {}=***", key);
                            std::env::set_var(key, value);
                        }

                        match DataflowController::new(&dataflow_path) {
                            Ok(mut controller) => {
                                // Pass env vars to controller so they're explicitly added to dora start command
                                controller.set_envs(env_vars.clone());

                                let mut disp = DynamicNodeDispatcher::new(controller);

                                match disp.start() {
                                    Ok(dataflow_id) => {
                                        log::info!("Dataflow started: {}", dataflow_id);
                                        state.write().dataflow_running = true;
                                        state.write().dataflow_id = Some(dataflow_id.clone());
                                        dataflow_start_time = Some(std::time::Instant::now());
                                        let _ = event_tx.send(DoraEvent::DataflowStarted { dataflow_id });
                                        dispatcher = Some(disp);
                                    }
                                    Err(e) => {
                                        log::error!("Failed to start dataflow: {}", e);
                                        let _ = event_tx.send(DoraEvent::Error {
                                            message: format!("Failed to start dataflow: {}", e),
                                        });
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to create controller: {}", e);
                                let _ = event_tx.send(DoraEvent::Error {
                                    message: format!("Failed to create controller: {}", e),
                                });
                            }
                        }
                    }

                    DoraCommand::StopDataflow => {
                        log::info!("Stopping dataflow (graceful)");
                        if let Some(mut disp) = dispatcher.take() {
                            if let Err(e) = disp.stop() {
                                log::error!("Failed to stop dataflow: {}", e);
                            }
                        }
                        state.write().dataflow_running = false;
                        state.write().dataflow_id = None;
                        dataflow_start_time = None;
                        let _ = event_tx.send(DoraEvent::DataflowStopped);
                    }

                    DoraCommand::StopDataflowWithGrace { grace_seconds } => {
                        log::info!("Stopping dataflow (grace: {}s)", grace_seconds);
                        if let Some(mut disp) = dispatcher.take() {
                            let duration = std::time::Duration::from_secs(grace_seconds);
                            if let Err(e) = disp.stop_with_grace_duration(duration) {
                                log::error!("Failed to stop dataflow: {}", e);
                            }
                        }
                        state.write().dataflow_running = false;
                        state.write().dataflow_id = None;
                        dataflow_start_time = None;
                        let _ = event_tx.send(DoraEvent::DataflowStopped);
                    }

                    DoraCommand::ForceStopDataflow => {
                        log::info!("Force stopping dataflow (immediate kill)");
                        if let Some(mut disp) = dispatcher.take() {
                            if let Err(e) = disp.force_stop() {
                                log::error!("Failed to force stop dataflow: {}", e);
                            }
                        }
                        state.write().dataflow_running = false;
                        state.write().dataflow_id = None;
                        dataflow_start_time = None;
                        let _ = event_tx.send(DoraEvent::DataflowStopped);
                    }

                    DoraCommand::SendPrompt { message } => {
                        if let Some(ref disp) = dispatcher {
                            if let Some(bridge) = disp.get_bridge("mofa-prompt-input") {
                                log::info!("Sending prompt via bridge: {}", message);
                                if let Err(e) = bridge.send("prompt", mofa_dora_bridge::DoraData::Text(message.clone())) {
                                    log::error!("Failed to send prompt: {}", e);
                                }
                            } else {
                                log::warn!("mofa-prompt-input bridge not found");
                            }
                        }
                    }

                    DoraCommand::SendControl { command } => {
                        if let Some(ref disp) = dispatcher {
                            if let Some(bridge) = disp.get_bridge("mofa-prompt-input") {
                                log::info!("Sending control command: {}", command);
                                let ctrl = mofa_dora_bridge::ControlCommand::new(&command);
                                if let Err(e) = bridge.send("control", mofa_dora_bridge::DoraData::Control(ctrl)) {
                                    log::error!("Failed to send control: {}", e);
                                }
                            } else {
                                log::warn!("mofa-prompt-input bridge not found for control");
                            }
                        }
                    }

                    DoraCommand::UpdateBufferStatus { fill_percentage } => {
                        state.write().buffer_fill = fill_percentage;
                        // Forward to audio player bridge for backpressure signaling to dora
                        if let Some(ref disp) = dispatcher {
                            if let Some(bridge) = disp.get_bridge("mofa-audio-player") {
                                if let Err(e) = bridge.send(
                                    "buffer_status",
                                    mofa_dora_bridge::DoraData::Json(serde_json::json!(fill_percentage)),
                                ) {
                                    log::debug!("Failed to send buffer status to bridge: {}", e);
                                }
                            }
                        }
                    }
                }
            }

            // Periodic status check - verify dataflow is actually running
            // Skip during startup grace period to avoid false positives
            let in_grace_period = dataflow_start_time
                .map(|t| t.elapsed() < startup_grace_period)
                .unwrap_or(false);

            if !in_grace_period && last_status_check.elapsed() >= status_check_interval {
                last_status_check = std::time::Instant::now();

                if let Some(ref disp) = dispatcher {
                    // Check if dataflow is still running via dora list
                    match disp.controller().read().get_status() {
                        Ok(status) => {
                            let was_running = state.read().dataflow_running;
                            let is_running = status.state.is_running();

                            if was_running && !is_running {
                                // Dataflow stopped unexpectedly
                                log::warn!("Dataflow stopped unexpectedly");
                                state.write().dataflow_running = false;
                                state.write().dataflow_id = None;
                                dataflow_start_time = None;
                                let _ = event_tx.send(DoraEvent::DataflowStopped);
                            }
                        }
                        Err(e) => {
                            log::debug!("Status check failed: {}", e);
                        }
                    }
                }
            }

            // Poll bridge events
            if let Some(ref disp) = dispatcher {
                for (node_id, bridge_event) in disp.poll_events() {
                    match bridge_event {
                        mofa_dora_bridge::BridgeEvent::Connected => {
                            log::info!("Bridge connected: {}", node_id);
                            let _ = event_tx.send(DoraEvent::BridgeConnected {
                                bridge_name: node_id.clone(),
                            });
                            // Update state based on bridge type
                            match node_id.as_str() {
                                "mofa-audio-player" => state.write().audio_player_connected = true,
                                "mofa-system-log" => state.write().system_log_connected = true,
                                "mofa-prompt-input" => state.write().prompt_input_connected = true,
                                _ => {}
                            }
                        }
                        mofa_dora_bridge::BridgeEvent::Disconnected => {
                            log::info!("Bridge disconnected: {}", node_id);
                            let _ = event_tx.send(DoraEvent::BridgeDisconnected {
                                bridge_name: node_id.clone(),
                            });
                            match node_id.as_str() {
                                "mofa-audio-player" => state.write().audio_player_connected = false,
                                "mofa-system-log" => state.write().system_log_connected = false,
                                "mofa-prompt-input" => state.write().prompt_input_connected = false,
                                _ => {}
                            }
                        }
                        mofa_dora_bridge::BridgeEvent::DataReceived { input_id, data, .. } => {
                            match data {
                                mofa_dora_bridge::DoraData::Audio(audio) => {
                                    let _ = event_tx.send(DoraEvent::AudioReceived { data: audio });
                                }
                                mofa_dora_bridge::DoraData::Chat(chat) => {
                                    state.write().pending_chat_messages.push(chat.clone());
                                    let _ = event_tx.send(DoraEvent::ChatReceived { message: chat });
                                }
                                mofa_dora_bridge::DoraData::Log(entry) => {
                                    state.write().pending_log_entries.push(entry.clone());
                                    let _ = event_tx.send(DoraEvent::LogReceived { entry });
                                }
                                mofa_dora_bridge::DoraData::Json(json) => {
                                    // JSON data from bridges (unused - LED visualization done in screen.rs)
                                    log::debug!("Received JSON from {}: input_id={}, data={:?}", node_id, input_id, json);
                                }
                                _ => {}
                            }
                        }
                        mofa_dora_bridge::BridgeEvent::Error(msg) => {
                            log::error!("Bridge error: {}", msg);
                            let _ = event_tx.send(DoraEvent::Error { message: msg });
                        }
                        _ => {}
                    }
                }
            }

            // Small sleep to avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Cleanup
        if let Some(mut disp) = dispatcher {
            let _ = disp.stop();
        }

        log::info!("Dora integration worker stopped");
    }
}

impl Drop for DoraIntegration {
    fn drop(&mut self) {
        // Send stop signal
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }

        // Wait for worker thread
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Default for DoraIntegration {
    fn default() -> Self {
        Self::new()
    }
}
