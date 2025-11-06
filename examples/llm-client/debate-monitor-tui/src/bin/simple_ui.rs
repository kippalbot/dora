use dora_node_api::arrow::array::AsArray;
use dora_node_api::dora_core::config::NodeId;
use dora_node_api::{DoraNode, Event, Parameter};
use std::collections::HashMap;
use std::io::{self, Write};

// Simple state to track messages
#[derive(Debug, Clone)]
struct ParticipantMessages {
    current_chunks: Vec<String>,
    completed_messages: Vec<String>,
    status: String,
}

impl ParticipantMessages {
    fn new() -> Self {
        Self {
            current_chunks: Vec::new(),
            completed_messages: Vec::new(),
            status: "idle".to_string(),
        }
    }

    fn add_chunk(&mut self, chunk: &str) {
        self.current_chunks.push(chunk.to_string());
    }

    fn complete_current(&mut self) {
        if !self.current_chunks.is_empty() {
            let message = self.current_chunks.join("");
            self.completed_messages.push(message);
            self.current_chunks.clear();
            self.status = "complete".to_string();
        }
    }

    fn set_status(&mut self, status: &str) {
        self.status = status.to_string();
    }

    fn get_latest_content(&self) -> String {
        let mut content = self.completed_messages.join("\n\n---\n\n");
        if !self.current_chunks.is_empty() {
            if !content.is_empty() {
                content.push_str("\n\n---\n\n");
            }
            content.push_str(&self.current_chunks.join(""));
        }
        content
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple Rust Debate Monitor ===");
    println!("Connecting to Dora dataflow...");

    // Initialize Dora node
    let (node, mut events) =
        match DoraNode::init_from_node_id(NodeId::from("debate-monitor".to_string())) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Failed to initialize Dora node: {:?}", e);
                return Ok(());
            }
        };

    println!("✓ Connected to dataflow");
    println!("Listening for debate messages...");
    println!("Press Ctrl+C to exit\n");

    // Track messages for each participant
    let mut participants: HashMap<String, ParticipantMessages> = [
        ("llm1".to_string(), ParticipantMessages::new()),
        ("llm2".to_string(), ParticipantMessages::new()),
        ("judge".to_string(), ParticipantMessages::new()),
    ]
    .iter()
    .cloned()
    .collect();

    // Main event loop
    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, data, metadata } => {
                let input_id = id.to_string();
                println!("📥 Received: {}", input_id);

                // Extract text
                let text_array = data.as_string::<i32>();
                let text = text_array
                    .iter()
                    .filter_map(|value| value.map(str::to_string))
                    .collect::<Vec<String>>()
                    .join(" ");

                // Map input ID to participant
                let participant = if input_id.starts_with("llm1_") {
                    "llm1"
                } else if input_id.starts_with("llm2_") {
                    "llm2"
                } else if input_id.starts_with("judge_") {
                    "judge"
                } else if input_id == "bundle_text" {
                    "judge"
                } else {
                    println!("  ⚠️  Unknown input ID: {}", input_id);
                    continue;
                };

                println!("  📍 Participant: {}", participant);

                // Handle different input types
                if input_id.ends_with("_text") || input_id == "bundle_text" {
                    println!(
                        "  📝 Text chunk ({} chars): {}",
                        text.len(),
                        if text.len() > 50 {
                            format!("{}...", &text[..50])
                        } else {
                            text.clone()
                        }
                    );

                    if let Some(participant_msgs) = participants.get_mut(participant) {
                        participant_msgs.add_chunk(&text);

                        // Check for completion signals
                        let is_complete = matches!(
                            metadata.parameters.get("is_complete"),
                            Some(Parameter::Bool(true))
                        );
                        let session_ended = matches!(
                            metadata.parameters.get("session_status"),
                            Some(Parameter::String(s)) if s == "ended"
                        );

                        if is_complete || session_ended {
                            println!("  ✅ Message completed for {}", participant);
                            participant_msgs.complete_current();
                            participant_msgs.set_status("complete");
                        } else {
                            participant_msgs.set_status("streaming");
                        }
                    }

                    // Display current state
                    display_current_state(&participants);
                } else if input_id.ends_with("_status") {
                    println!("  📊 Status update: {}", text);
                    if let Some(participant_msgs) = participants.get_mut(participant) {
                        participant_msgs.set_status(&text);
                    }
                }
            }
            Event::Stop(_) => {
                println!("⏹️  Dataflow stopped");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

fn display_current_state(participants: &HashMap<String, ParticipantMessages>) {
    println!("\n============================================================");
    println!("CURRENT DEBATE STATE");
    println!("============================================================");

    for (name, msgs) in participants {
        let display_name = match name.as_str() {
            "llm1" => "LLM1 (Daniu - Pro Side)",
            "llm2" => "LLM2 (Yifei - Con Side)",
            "judge" => "Judge (Moderator)",
            _ => name,
        };

        println!("\n--- {} [{}] ---", display_name, msgs.status);
        let content = msgs.get_latest_content();
        if !content.is_empty() {
            println!("{}", content);
        } else {
            println!("(No messages yet)");
        }
    }

    println!("\n============================================================\n");
    io::stdout().flush().unwrap();
}
