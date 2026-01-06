//! Audio player bridge
//!
//! Connects to dora as `mofa-audio-player` dynamic node.
//! Receives audio from TTS nodes and provides:
//! - Audio samples to the widget for playback
//! - Buffer status output back to dora
//! - Participant audio levels for LED visualization (consolidated from participant_panel)

use crate::bridge::{BridgeEvent, BridgeState, DoraBridge};
use crate::data::{AudioData, DoraData, EventMetadata};
use crate::error::{BridgeError, BridgeResult};
use arrow::array::Array;
use crossbeam_channel::{bounded, Receiver, Sender};
use dora_node_api::{DoraNode, Event, IntoArrow, dora_core::config::{NodeId, DataId}, Parameter};
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread;
use tracing::{debug, error, info, warn};

// NOTE: LED visualization (band levels) is calculated in screen.rs from output waveform
// This is more accurate since it reflects what's actually being played,
// not what's being received (which may be buffered ahead of playback)

/// Audio player bridge - receives audio from dora, provides to widget
pub struct AudioPlayerBridge {
    /// Node ID (e.g., "mofa-audio-player")
    node_id: String,
    /// Current state
    state: Arc<RwLock<BridgeState>>,
    /// Event sender to widget
    event_sender: Sender<BridgeEvent>,
    /// Event receiver for widget
    event_receiver: Receiver<BridgeEvent>,
    /// Audio data sender to widget
    audio_sender: Sender<AudioData>,
    /// Audio data receiver for widget
    audio_receiver: Receiver<AudioData>,
    /// Buffer status sender from widget
    buffer_status_sender: Sender<f64>,
    /// Buffer status receiver for dora
    buffer_status_receiver: Receiver<f64>,
    /// Stop signal
    stop_sender: Option<Sender<()>>,
    /// Worker thread handle
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl AudioPlayerBridge {
    /// Create a new audio player bridge
    pub fn new(node_id: &str) -> Self {
        let (event_tx, event_rx) = bounded(100);
        // Use larger buffer to prevent blocking which would stall the pipeline
        let (audio_tx, audio_rx) = bounded(500);
        let (buffer_tx, buffer_rx) = bounded(10);

        Self {
            node_id: node_id.to_string(),
            state: Arc::new(RwLock::new(BridgeState::Disconnected)),
            event_sender: event_tx,
            event_receiver: event_rx,
            audio_sender: audio_tx,
            audio_receiver: audio_rx,
            buffer_status_sender: buffer_tx,
            buffer_status_receiver: buffer_rx,
            stop_sender: None,
            worker_handle: None,
        }
    }

    /// Get receiver for audio data (widget uses this)
    pub fn audio_receiver(&self) -> Receiver<AudioData> {
        self.audio_receiver.clone()
    }

    /// Send buffer status back to dora (widget calls this)
    pub fn send_buffer_status(&self, fill_percentage: f64) -> BridgeResult<()> {
        self.buffer_status_sender
            .send(fill_percentage)
            .map_err(|_| BridgeError::ChannelSendError)
    }

    /// Run the dora event loop in background thread
    fn run_event_loop(
        node_id: String,
        state: Arc<RwLock<BridgeState>>,
        event_sender: Sender<BridgeEvent>,
        audio_sender: Sender<AudioData>,
        buffer_status_receiver: Receiver<f64>,
        stop_receiver: Receiver<()>,
    ) {
        info!("Starting audio player bridge event loop for {}", node_id);

        // Initialize dora node
        let (mut node, mut events) = match DoraNode::init_from_node_id(NodeId::from(node_id.clone())) {
            Ok(n) => n,
            Err(e) => {
                error!("Failed to init dora node {}: {}", node_id, e);
                *state.write() = BridgeState::Error;
                let _ = event_sender.send(BridgeEvent::Error(format!("Init failed: {}", e)));
                return;
            }
        };

        *state.write() = BridgeState::Connected;
        let _ = event_sender.send(BridgeEvent::Connected);

        // Session tracking - track which question_ids we've sent session_start for
        // to avoid flooding the controller with duplicate signals
        let mut session_start_sent_for: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Active participant tracking for LED visualization
        let mut active_participant: Option<String> = None;
        let mut active_switch_for: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Event loop
        loop {
            // Check for stop signal
            if stop_receiver.try_recv().is_ok() {
                info!("Audio player bridge received stop signal");
                break;
            }

            // Forward buffer status from UI's AudioPlayer to dora
            // The actual buffer fill percentage comes from CircularAudioBuffer::fill_percentage()
            // in the UI layer, sent here via channel every 50ms
            while let Ok(status) = buffer_status_receiver.try_recv() {
                if let Err(e) = Self::send_buffer_status_to_dora(&mut node, status) {
                    warn!("Failed to send buffer status: {}", e);
                } else {
                    debug!("Buffer status: {:.1}%", status);
                }
            }

            // Receive dora events with timeout
            match events.recv_timeout(std::time::Duration::from_millis(100)) {
                Some(event) => {
                    Self::handle_dora_event(
                        event,
                        &mut node,
                        &audio_sender,
                        &event_sender,
                        &mut session_start_sent_for,
                        &mut active_participant,
                        &mut active_switch_for,
                    );
                }
                None => {
                    // Timeout or no event, continue
                }
            }
        }

        *state.write() = BridgeState::Disconnected;
        let _ = event_sender.send(BridgeEvent::Disconnected);
        info!("Audio player bridge event loop ended");
    }

    /// Handle a dora event
    fn handle_dora_event(
        event: Event,
        node: &mut DoraNode,
        audio_sender: &Sender<AudioData>,
        event_sender: &Sender<BridgeEvent>,
        session_start_sent_for: &mut std::collections::HashSet<String>,
        active_participant: &mut Option<String>,
        active_switch_for: &mut std::collections::HashSet<String>,
    ) {
        match event {
            Event::Input { id, data, metadata } => {
                let input_id = id.as_str();

                // Extract metadata (handle all parameter types like conference-dashboard)
                let mut event_meta = EventMetadata::default();
                for (key, value) in metadata.parameters.iter() {
                    let string_value = match value {
                        Parameter::String(s) => s.clone(),
                        Parameter::Integer(i) => i.to_string(),
                        Parameter::Float(f) => f.to_string(),
                        Parameter::Bool(b) => b.to_string(),
                        Parameter::ListInt(l) => format!("{:?}", l),
                        Parameter::ListFloat(l) => format!("{:?}", l),
                        Parameter::ListString(l) => format!("{:?}", l),
                    };
                    event_meta.values.insert(key.clone(), string_value);
                }

                // Handle audio inputs
                if input_id.contains("audio") {
                    if let Some(audio_data) = Self::extract_audio(&data, &event_meta) {
                        let sample_count = audio_data.samples.len();
                        debug!(
                            "Received audio: {} samples, {}Hz from {}",
                            sample_count,
                            audio_data.sample_rate,
                            input_id
                        );

                        // Extract participant ID from input_id (e.g., "audio_student1" -> "student1")
                        let participant_id = input_id
                            .strip_prefix("audio_")
                            .unwrap_or("unknown")
                            .to_string();

                        // Get question_id from metadata
                        let question_id = event_meta.get("question_id");

                        // Send session_start ONCE per question_id on FIRST audio chunk
                        // (matching conference-dashboard behavior: send on first audio OR when session_status="started")
                        // This marks when audio playback begins for a new LLM/TTS response
                        // The controller waits for this signal to advance to the next speaker
                        if let Some(qid) = question_id {
                            // Only send if we haven't sent for this question_id yet
                            if !session_start_sent_for.contains(qid) {
                                if let Err(e) = Self::send_session_start(node, input_id, &event_meta) {
                                    warn!("Failed to send session_start: {}", e);
                                } else {
                                    info!("Session started for question_id={} (first audio chunk)", qid);
                                    session_start_sent_for.insert(qid.to_string());

                                    // Keep the set size bounded (only track last 100 question_ids)
                                    if session_start_sent_for.len() > 100 {
                                        // Remove oldest entries (approximation)
                                        let to_remove: Vec<_> = session_start_sent_for.iter()
                                            .take(50)
                                            .cloned()
                                            .collect();
                                        for key in to_remove {
                                            session_start_sent_for.remove(&key);
                                        }
                                    }
                                }
                            }

                            // Switch active speaker ONCE per question_id (for logging)
                            if !active_switch_for.contains(qid) {
                                if active_participant.as_ref() != Some(&participant_id) {
                                    *active_participant = Some(participant_id.clone());
                                    debug!("Active speaker changed to: {}", participant_id);
                                }
                                active_switch_for.insert(qid.to_string());

                                // Keep the set size bounded
                                if active_switch_for.len() > 100 {
                                    let to_remove: Vec<_> = active_switch_for.iter()
                                        .take(50)
                                        .cloned()
                                        .collect();
                                    for key in to_remove {
                                        active_switch_for.remove(&key);
                                    }
                                }
                            }
                        }

                        // NOTE: LED visualization is calculated in screen.rs from output waveform
                        // (more accurate since it reflects what's actually being played)
                        // The bridge only tracks active speaker for session management

                        // IMPORTANT: Override participant_id from input_id (more reliable than metadata)
                        // input_id is "audio_student1" -> participant_id is "student1"
                        let mut audio_data_with_participant = audio_data.clone();
                        audio_data_with_participant.participant_id = Some(participant_id.clone());

                        // Use try_send() to avoid blocking if channel is full
                        // Blocking here would prevent session_start/audio_complete from being sent
                        if let Err(e) = audio_sender.try_send(audio_data_with_participant.clone()) {
                            warn!("Audio channel full, dropping audio chunk: {}", e);
                        }
                        let _ = event_sender.try_send(BridgeEvent::DataReceived {
                            input_id: input_id.to_string(),
                            data: DoraData::Audio(audio_data_with_participant),
                            metadata: event_meta.clone(),
                        });

                        // Send audio_complete signal back to text-segmenter
                        // This allows the next segment to be released
                        // CRITICAL: This must be sent for every audio chunk to keep the pipeline flowing
                        if let Err(e) = Self::send_audio_complete(node, input_id, &event_meta) {
                            warn!("Failed to send audio_complete: {}", e);
                        } else {
                            debug!("Sent audio_complete for {} (qid={:?})", input_id, event_meta.get("question_id"));
                        }
                    }
                }
            }
            Event::Stop(_) => {
                info!("Received stop event from dora");
            }
            _ => {}
        }
    }

    // NOTE: Audio level and band calculation removed - now done in screen.rs from output waveform
    // This is more accurate since it reflects what's actually being played,
    // not what's being received (which may be buffered ahead of playback)

    /// Send audio_complete signal to notify text-segmenter that audio was received
    /// Matches conference-dashboard's implementation for compatibility
    fn send_audio_complete(node: &mut DoraNode, input_id: &str, metadata: &EventMetadata) -> BridgeResult<()> {
        use std::collections::BTreeMap;

        // Extract participant from input_id (e.g., "audio_student1" -> "student1")
        let participant = input_id.strip_prefix("audio_").unwrap_or(input_id);

        // Build metadata with participant info (matching conference-dashboard format)
        let mut params: BTreeMap<String, Parameter> = BTreeMap::new();
        params.insert(
            "participant".to_string(),
            Parameter::String(participant.to_string()),
        );

        // Include question_id if present in incoming metadata
        if let Some(qid) = metadata.get("question_id") {
            params.insert(
                "question_id".to_string(),
                Parameter::String(qid.to_string()),
            );
        }

        // Include session_status if present in incoming metadata
        if let Some(status) = metadata.get("session_status") {
            params.insert(
                "session_status".to_string(),
                Parameter::String(status.to_string()),
            );
        }

        // Use vec!["received"] format to match conference-dashboard
        let data = vec!["received".to_string()].into_arrow();
        let output_id: DataId = "audio_complete".to_string().into();

        debug!("Sending audio_complete for participant: {} (question_id={:?}, session_status={:?})",
            participant, metadata.get("question_id"), metadata.get("session_status"));

        node.send_output(output_id, params, data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }

    /// Send session_start signal to notify conference-controller that audio playback has begun
    /// This is critical for the controller to advance to the next speaker
    fn send_session_start(node: &mut DoraNode, input_id: &str, metadata: &EventMetadata) -> BridgeResult<()> {
        use std::collections::BTreeMap;

        // Extract participant from input_id (e.g., "audio_student1" -> "student1")
        let participant = input_id.strip_prefix("audio_").unwrap_or(input_id);

        // Build metadata (matching conference-dashboard format)
        let mut params: BTreeMap<String, Parameter> = BTreeMap::new();

        // Include question_id - REQUIRED by conference-controller
        if let Some(qid) = metadata.get("question_id") {
            params.insert(
                "question_id".to_string(),
                Parameter::String(qid.to_string()),
            );
        }

        params.insert(
            "participant".to_string(),
            Parameter::String(participant.to_string()),
        );

        params.insert(
            "source".to_string(),
            Parameter::String("mofa-audio-player".to_string()),
        );

        // Include session_status if present
        if let Some(status) = metadata.get("session_status") {
            params.insert(
                "session_status".to_string(),
                Parameter::String(status.to_string()),
            );
        }

        // Use vec!["audio_started"] format to match conference-dashboard
        let data = vec!["audio_started".to_string()].into_arrow();
        let output_id: DataId = "session_start".to_string().into();

        info!("Sending session_start for participant: {} (question_id={:?})",
            participant, metadata.get("question_id"));

        node.send_output(output_id, params, data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }

    /// Extract audio data from dora arrow data
    /// Handles multiple formats: Float32, Float64, Int16, ListArray, LargeListArray
    fn extract_audio(data: &dora_node_api::ArrowData, metadata: &EventMetadata) -> Option<AudioData> {
        use arrow::array::{Float32Array, Float64Array, Int16Array, ListArray, LargeListArray};
        use arrow::datatypes::DataType;

        let array = &data.0;
        if array.is_empty() {
            return None;
        }

        // Try to extract f32 array
        let samples: Vec<f32> = match array.data_type() {
            DataType::Float32 => {
                array.as_any().downcast_ref::<Float32Array>()
                    .map(|arr| arr.values().to_vec())?
            }
            DataType::Float64 => {
                array.as_any().downcast_ref::<Float64Array>()
                    .map(|arr| arr.values().iter().map(|&x| x as f32).collect())?
            }
            DataType::Int16 => {
                array.as_any().downcast_ref::<Int16Array>()
                    .map(|arr| arr.values().iter().map(|&x| x as f32 / 32768.0).collect())?
            }
            // Handle ListArray<Float32> - primespeech sends pa.array([audio_array])
            DataType::List(_) | DataType::LargeList(_) => {
                debug!("Audio data is ListArray, extracting inner array");

                // Try ListArray first
                if let Some(list_arr) = array.as_any().downcast_ref::<ListArray>() {
                    if list_arr.len() > 0 {
                        let first_value = list_arr.value(0);
                        if let Some(float_arr) = first_value.as_any().downcast_ref::<Float32Array>() {
                            debug!("Extracted {} f32 samples from ListArray", float_arr.len());
                            float_arr.values().to_vec()
                        } else if let Some(float_arr) = first_value.as_any().downcast_ref::<Float64Array>() {
                            debug!("Extracted {} f64 samples from ListArray", float_arr.len());
                            float_arr.values().iter().map(|&v| v as f32).collect()
                        } else {
                            warn!("ListArray inner type not Float32/Float64: {:?}", first_value.data_type());
                            return None;
                        }
                    } else {
                        return None;
                    }
                } else if let Some(list_arr) = array.as_any().downcast_ref::<LargeListArray>() {
                    if list_arr.len() > 0 {
                        let first_value = list_arr.value(0);
                        if let Some(float_arr) = first_value.as_any().downcast_ref::<Float32Array>() {
                            debug!("Extracted {} f32 samples from LargeListArray", float_arr.len());
                            float_arr.values().to_vec()
                        } else if let Some(float_arr) = first_value.as_any().downcast_ref::<Float64Array>() {
                            debug!("Extracted {} f64 samples from LargeListArray", float_arr.len());
                            float_arr.values().iter().map(|&v| v as f32).collect()
                        } else {
                            warn!("LargeListArray inner type not Float32/Float64: {:?}", first_value.data_type());
                            return None;
                        }
                    } else {
                        return None;
                    }
                } else {
                    warn!("Failed to extract audio from ListArray");
                    return None;
                }
            }
            dt => {
                warn!("Unsupported audio data type: {:?}", dt);
                return None;
            }
        };

        if samples.is_empty() {
            return None;
        }

        // Get sample rate from metadata or use default
        let sample_rate = metadata
            .get("sample_rate")
            .and_then(|s| s.parse().ok())
            .unwrap_or(32000);

        let participant_id = metadata.participant_id().map(|s| s.to_string());

        Some(AudioData {
            samples,
            sample_rate,
            channels: 1,
            participant_id,
        })
    }

    /// Send buffer status to dora
    fn send_buffer_status_to_dora(node: &mut DoraNode, status: f64) -> BridgeResult<()> {
        let data = vec![status].into_arrow();
        let output_id: DataId = "buffer_status".to_string().into();
        node.send_output(output_id, Default::default(), data)
            .map_err(|e| BridgeError::SendFailed(e.to_string()))
    }
}

impl DoraBridge for AudioPlayerBridge {
    fn node_id(&self) -> &str {
        &self.node_id
    }

    fn state(&self) -> BridgeState {
        *self.state.read()
    }

    fn connect(&mut self) -> BridgeResult<()> {
        if self.is_connected() {
            return Err(BridgeError::AlreadyConnected);
        }

        *self.state.write() = BridgeState::Connecting;

        let (stop_tx, stop_rx) = bounded(1);
        self.stop_sender = Some(stop_tx);

        let node_id = self.node_id.clone();
        let state = Arc::clone(&self.state);
        let event_sender = self.event_sender.clone();
        let audio_sender = self.audio_sender.clone();
        let buffer_receiver = self.buffer_status_receiver.clone();

        let handle = thread::spawn(move || {
            Self::run_event_loop(
                node_id,
                state,
                event_sender,
                audio_sender,
                buffer_receiver,
                stop_rx,
            );
        });

        self.worker_handle = Some(handle);

        // Wait briefly for connection
        std::thread::sleep(std::time::Duration::from_millis(200));

        Ok(())
    }

    fn disconnect(&mut self) -> BridgeResult<()> {
        if let Some(stop_tx) = self.stop_sender.take() {
            let _ = stop_tx.send(());
        }

        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }

        *self.state.write() = BridgeState::Disconnected;
        Ok(())
    }

    fn send(&self, output_id: &str, data: DoraData) -> BridgeResult<()> {
        if !self.is_connected() {
            return Err(BridgeError::NotConnected);
        }

        match (output_id, data) {
            ("buffer_status", DoraData::Json(val)) => {
                if let Some(status) = val.as_f64() {
                    self.buffer_status_sender
                        .send(status)
                        .map_err(|_| BridgeError::ChannelSendError)?;
                }
            }
            _ => {
                warn!("Unknown output: {}", output_id);
            }
        }

        Ok(())
    }

    fn subscribe(&self) -> Receiver<BridgeEvent> {
        self.event_receiver.clone()
    }

    fn expected_inputs(&self) -> Vec<String> {
        vec![
            "audio".to_string(),
            "audio_student1".to_string(),
            "audio_student2".to_string(),
            "audio_tutor".to_string(),
        ]
    }

    fn expected_outputs(&self) -> Vec<String> {
        vec![
            "buffer_status".to_string(),
            "status".to_string(),
            "session_start".to_string(),
            "audio_complete".to_string(),
            "log".to_string(),
        ]
    }
}

impl Drop for AudioPlayerBridge {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}
