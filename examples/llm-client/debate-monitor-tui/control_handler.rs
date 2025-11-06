// Handle control messages by sending them to the Dora dataflow
fn handle_control_messages(
    mut node: DoraNode,
    mut rx_control: tokio_mpsc::UnboundedReceiver<String>,
) {
    while let Some(message) = rx_control.blocking_recv() {
        // Check if it's a reset command
        if message.trim().to_lowercase() == "reset" {
            // Send reset command to bridge_control output
            let payload = r#"{"command": "reset"}"#;
            if let Err(e) = node.send_output(
                DataId::from("bridge_control".to_string()),
                Default::default(),
                StringArray::from(vec![payload]),
            ) {
                eprintln!("Failed to send reset command: {:?}", e);
            }
        } else {
            // Send regular prompt to control output
            let payload = format!(r#"{{"prompt": "{}"}}"#, message);
            if let Err(e) = node.send_output(
                DataId::from("control".to_string()),
                Default::default(),
                StringArray::from(vec![payload.as_str()]),
            ) {
                eprintln!("Failed to send control message: {:?}", e);
            }
        }
    }
}