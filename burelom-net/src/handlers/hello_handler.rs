use core::cell::RefCell;
use crate::node_registry::NodeRegistry;
use crate::proto;
use alloc::sync::Arc;
use crate::logging::{info};
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use crate::packetizer::Packetizer;
use crate::traits::mac::Mac;
use crate::burelom_node::{BurelomNode};
use crate::prelude::*;

/// Hello packets handler
pub struct HelloHandler {
  addr: u32,
  packetizer: Arc<Packetizer>,
  mac:Arc<Box<dyn Mac>>,
  seen_packet: Arc<Mutex<CriticalSectionRawMutex, RefCell<Vec<(u32, u32)>>>>,
  node_registry: Arc<NodeRegistry>
}

impl HelloHandler {
  pub fn new(node: &BurelomNode) -> Self {
    HelloHandler {
        addr: node.addr,
        packetizer: node.packetizer.clone(),
        mac: node.mac.clone(),
        seen_packet: node.seen_packet.clone(),
        node_registry: node.node_registry.clone()
    }
  }

  /// Handles packets of type 'hello'. Basic logic. Handler updates node registry with
  /// data provided from hello packet and memorizes pakcet id for prevent last
  /// packet hadnling. Than rebradcast hello throug mac layer to neighboard.
  /// 
  /// #Arguments
  /// `packet` - processed packet
  pub async fn handle(&self, packet: &mut proto::Packet) {
    let Some(proto::packet::Body::Hello(hello)) = packet.body.as_mut() else {
        unreachable!("handle called for non-Hello packet");
    };

    // Check if we early seen this hello - drop it
    let seen = self.seen_packet.lock(|seen_packet| {
      seen_packet.borrow()
      .iter()
      .any(|(id, _)| packet.id == *id)
    });
     
    if seen {
      info!("<{}> Received transit hello packet from={}, packet dropped due seen",
        self.addr, packet.source);
      return;
    }
    // Add hello to seen
    self.seen_packet.lock(|seen_packet| {
      seen_packet.borrow_mut().push((packet.id, 0));
    });

    // If we are hello sender, drop packet
    if packet.source == self.addr {
      return;
    }

    // Handle hello packet
    info!("<{}> Received target hello packet from={}", self.addr, packet.source);
    
    // Add data to node registr
    self.node_registry.update_from_hello(hello, packet.source, packet.hops);

    // Rebroadcast packet
    packet.hops = packet.hops + 1;
    let (_, bin) = self.packetizer.encode_packet(packet);
    self.mac.send(0, &bin).await;
  }
}