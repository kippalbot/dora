use dora_node_api::arrow::array::AsArray;
use dora_node_api::dora_core::config::NodeId;
use dora_node_api::{DoraNode, Event, Parameter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Rust Debate Monitor Diagnostic ===");
    println!("Connecting to Dora dataflow as 'debate-monitor' node...");

    // Initialize Dora node as a dynamic node
    let (node, mut events) = match DoraNode::init_from_node_id(NodeId::from(
        "debate-monitor".to_string(),
    )) {
        Ok(result) => {
            println!("✓ Successfully connected to Dora dataflow");
            result
        }
        Err(e) => {
            eprintln!("✗ Failed to initialize Dora node: {:?}", e);
            eprintln!(
                "Make sure the dataflow is running and contains a node with id 'debate-monitor'"
            );
            return Ok(());
        }
    };

    println!("\nListening for events...");
    println!("Press Ctrl+C to exit.\n");

    // Listen for events and display diagnostic information
    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, data, metadata } => {
                println!("=== INPUT EVENT RECEIVED ===");
                println!("Input ID: {}", id);

                // Extract text from the data
                let text_array = data.as_string::<i32>();
                let text = text_array
                    .iter()
                    .filter_map(|value| value.map(str::to_string))
                    .collect::<Vec<String>>()
                    .join(" ");

                println!(
                    "Text content: {} ({} chars)",
                    if text.len() > 100 {
                        format!("{}...", &text[..100])
                    } else {
                        text.clone()
                    },
                    text.len()
                );

                // Show metadata parameters
                println!("Metadata parameters:");
                for (key, value) in &metadata.parameters {
                    match value {
                        Parameter::String(s) => println!("  {}: String({})", key, s),
                        Parameter::Bool(b) => println!("  {}: Bool({})", key, b),
                        _ => println!("  {}: {:?}", key, value),
                    }
                }

                // Check for completion signals
                if let Some(Parameter::Bool(true)) = metadata.parameters.get("is_complete") {
                    println!("➡️  COMPLETION SIGNAL DETECTED");
                }

                if let Some(Parameter::String(status)) = metadata.parameters.get("session_status") {
                    if status == "ended" {
                        println!("⏹️  SESSION ENDED SIGNAL DETECTED");
                    }
                }

                println!();
            }
            Event::Stop(_) => {
                println!("⏹️  Dataflow stopped");
                break;
            }
            _ => {
                println!("Received other event type: {:?}", event);
            }
        }
    }

    println!("Diagnostic monitor exiting.");
    Ok(())
}
