//! DoraBridge trait and common bridge functionality

use crate::data::{DoraData, EventMetadata};
use crate::error::BridgeResult;
use crossbeam_channel::{Receiver, Sender};
use std::sync::Arc;
use parking_lot::RwLock;

/// Bridge connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeState {
    /// Bridge is disconnected
    Disconnected,
    /// Bridge is connecting
    Connecting,
    /// Bridge is connected and ready
    Connected,
    /// Bridge is disconnecting
    Disconnecting,
    /// Bridge encountered an error
    Error,
}

impl Default for BridgeState {
    fn default() -> Self {
        BridgeState::Disconnected
    }
}

/// Events from the bridge to the widget
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    /// Bridge connected successfully
    Connected,
    /// Bridge disconnected
    Disconnected,
    /// Data received from dora
    DataReceived {
        input_id: String,
        data: DoraData,
        metadata: EventMetadata,
    },
    /// Error occurred
    Error(String),
    /// State changed
    StateChanged(BridgeState),
}

/// Handler for incoming data from dora
pub type InputHandler = Box<dyn Fn(DoraData, EventMetadata) + Send + Sync>;

/// Core trait for all dora bridges
/// Each widget implements this trait to connect as a dynamic node
pub trait DoraBridge: Send + Sync {
    /// Get the node ID for this bridge (e.g., "mofa-audio-player")
    fn node_id(&self) -> &str;

    /// Get current connection state
    fn state(&self) -> BridgeState;

    /// Connect to the dora dataflow as a dynamic node
    fn connect(&mut self) -> BridgeResult<()>;

    /// Disconnect from dora
    fn disconnect(&mut self) -> BridgeResult<()>;

    /// Check if connected
    fn is_connected(&self) -> bool {
        self.state() == BridgeState::Connected
    }

    /// Send data to a dora output
    fn send(&self, output_id: &str, data: DoraData) -> BridgeResult<()>;

    /// Subscribe to events from the bridge
    fn subscribe(&self) -> Receiver<BridgeEvent>;

    /// Get list of input IDs this bridge expects
    fn expected_inputs(&self) -> Vec<String>;

    /// Get list of output IDs this bridge provides
    fn expected_outputs(&self) -> Vec<String>;
}

/// Shared state for bridge communication with widgets
#[derive(Debug)]
pub struct BridgeSharedState<T> {
    inner: Arc<RwLock<T>>,
}

impl<T> BridgeSharedState<T> {
    pub fn new(state: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(state)),
        }
    }

    pub fn read(&self) -> parking_lot::RwLockReadGuard<T> {
        self.inner.read()
    }

    pub fn write(&self) -> parking_lot::RwLockWriteGuard<T> {
        self.inner.write()
    }
}

impl<T> Clone for BridgeSharedState<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Channel pair for bidirectional communication
pub struct BridgeChannel<T> {
    pub sender: Sender<T>,
    pub receiver: Receiver<T>,
}

impl<T> BridgeChannel<T> {
    pub fn new() -> (Self, Self) {
        let (tx1, rx1) = crossbeam_channel::unbounded();
        let (tx2, rx2) = crossbeam_channel::unbounded();
        (
            BridgeChannel {
                sender: tx1,
                receiver: rx2,
            },
            BridgeChannel {
                sender: tx2,
                receiver: rx1,
            },
        )
    }
}

/// Builder for creating bridges with common configuration
pub struct BridgeBuilder {
    node_id: String,
    inputs: Vec<String>,
    outputs: Vec<String>,
}

impl BridgeBuilder {
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn with_input(mut self, input_id: impl Into<String>) -> Self {
        self.inputs.push(input_id.into());
        self
    }

    pub fn with_output(mut self, output_id: impl Into<String>) -> Self {
        self.outputs.push(output_id.into());
        self
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn inputs(&self) -> &[String] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[String] {
        &self.outputs
    }
}
