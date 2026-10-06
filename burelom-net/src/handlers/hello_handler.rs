use alloc::sync::Arc;
use crate::node_registry::NodeRegistry;
use crate::burelom_node::{BurelomNode};

/// Hello packets handler
pub struct HelloHandler {
  node_registry: Arc<NodeRegistry>
}

impl HelloHandler {
  pub fn new(node: &BurelomNode) -> Self {
    HelloHandler {
        node_registry: node.node_registry.clone()
    }
  }

  /// Handles packets of type 'becon'. Basic logic. Beacase hello packets is 
  /// always goes from the neares node (in direct rf visibility), we have
  /// information about rssi. This handler updates node registry whith this
  /// information
  /// 
  /// #Arguments
  /// `gateway` - where are packet from
  /// `rssi` - strange of signal on rf link
  pub async fn handle(&self, gateway: u32, rssi: i32) {
    // Update rssi data in node registry
    self.node_registry.update_from_hello(gateway, rssi);
  }
}