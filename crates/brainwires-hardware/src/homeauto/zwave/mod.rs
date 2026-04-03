pub mod command_class;
pub mod serial_api;
pub mod types;

use async_trait::async_trait;

use super::error::HomeAutoResult;
use super::types::HomeAutoEvent;
use super::BoxStream;
pub use command_class::CommandClass;
pub use serial_api::ZWaveSerialController;
pub use types::{NodeId, ZWaveNode, ZWaveNodeKind};

/// Abstraction over a Z-Wave network controller.
///
/// Implemented by [`ZWaveSerialController`] (Z-Wave Serial API over USB stick).
#[async_trait]
pub trait ZWaveController: Send + Sync {
    /// Open the serial port and initialise the controller.
    async fn start(&self) -> HomeAutoResult<()>;

    /// Close the serial port and shut down.
    async fn stop(&self) -> HomeAutoResult<()>;

    /// Open an inclusion (join) window for up to `timeout_secs` seconds.
    /// Returns the newly included node, or an error if no node joined in time.
    async fn include_node(&self, timeout_secs: u8) -> HomeAutoResult<ZWaveNode>;

    /// Open an exclusion window to remove a node from the network.
    async fn exclude_node(&self, timeout_secs: u8) -> HomeAutoResult<()>;

    /// Return a snapshot of all known nodes.
    async fn nodes(&self) -> HomeAutoResult<Vec<ZWaveNode>>;

    /// Transmit a Z-Wave command class frame to a specific node.
    /// `data` is the payload *after* the Command Class ID byte (which is taken from `cc`).
    async fn send_cc(
        &self,
        node_id: NodeId,
        cc: CommandClass,
        data: &[u8],
    ) -> HomeAutoResult<()>;

    /// Subscribe to a stream of events from this controller.
    fn events(&self) -> BoxStream<'static, HomeAutoEvent>;
}
