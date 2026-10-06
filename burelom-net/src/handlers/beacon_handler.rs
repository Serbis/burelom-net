use core::cell::RefCell;
use crate::node_registry::NodeRegistry;
use crate::proto;
use alloc::sync::Arc;
use heapless::HistoryBuf;
use crate::logging::{info};
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use crate::packetizer::Packetizer;
use crate::traits::mac::Mac;
use crate::burelom_node::{BurelomNode};
use crate::prelude::*;

/// Beacon packets handler
pub struct BeaconHandler {
  addr: u32,
  packetizer: Arc<Packetizer>,
  mac:Arc<Box<dyn Mac>>,
  seen_packet: Arc<Mutex<CriticalSectionRawMutex, RefCell<HistoryBuf<u32, 10>>>>,
  node_registry: Arc<NodeRegistry>
}

impl BeaconHandler {
  pub fn new(node: &BurelomNode) -> Self {
    BeaconHandler {
        addr: node.addr,
        packetizer: node.packetizer.clone(),
        mac: node.mac.clone(),
        seen_packet: node.seen_packet.clone(),
        node_registry: node.node_registry.clone()
    }
  }

  /// Handles packets of type 'becon'. Basic logic. Handler updates node registry with
  /// data provided from beacon packet and memorizes pakcet id for prevent last
  /// packet hadnling. Than rebradcast beacon throug mac layer to neighboard.
  /// 
  /// #Arguments
  /// `packet` - processed packet
  pub async fn handle(&self, packet: &mut proto::Packet) {
    let Some(proto::packet::Body::Beacon(beacon)) = packet.body.as_mut() else {
        unreachable!("handle called for non-Beacon packet");
    };

    // Check if we early seen this beacon - drop it
    let seen = self.seen_packet.lock(|seen_packet| {
      seen_packet.borrow()
      .iter()
      .any(|id| packet.id == *id)
    });
     
    if seen {
      info!("<{}> Received transit beacon packet from={}, packet dropped due seen",
        self.addr, packet.source);
      return;
    }
    // Add beacon to seen
    self.seen_packet.lock(|seen_packet| {
      seen_packet.borrow_mut().write(packet.id);
    });

    // If we are beacon sender, drop packet
    if packet.source == self.addr {
      return;
    }

    // Handle beacon packet
    info!("<{}> Received target beacon packet from={}", self.addr, packet.source);
    
    // Add data to node registr
    self.node_registry.update_from_beacon(beacon, packet.source, packet.hops);

    // Rebroadcast packet
    packet.hops = packet.hops + 1;
    let (_, bin) = self.packetizer.encode_packet(packet);
    self.mac.send(0, &bin).await;
  }
}