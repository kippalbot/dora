// This is the simple pattern that works (like the Python version):
//
// 1. Create single DoraNode instance
// 2. Use single event loop
// 3. Handle both receiving events and sending messages
//
// Simple example:
//
// fn main() -> Result<()> {
//     // Single node for both receiving and sending
//     let (mut node, mut events) = DoraNode::init_from_node_id(NodeId::from("debate-monitor".to_string()))?;
//
//     loop {
//         // Non-blocking check for events
//         match events.recv() {
//             Ok(Event::Input { id, data, .. }) => {
//                 // Handle incoming messages
//                 // ... process data ...
//             }
//             Ok(Event::Stop(_)) => break,
//             _ => {}
//         }
//
//         // Check for user input/control messages
//         // When user wants to send a message:
//         let payload = r#"{"prompt": "Hello"}"#;
//         node.send_output(
//             DataId::from("control".to_string()),
//             Default::default(),
//             StringArray::from(vec![payload]),
//         )?;
//     }
//
//     Ok(())
// }
//
// The key is using the SAME node instance for both receiving and sending,
// just like the Python version does with its single Node instance.