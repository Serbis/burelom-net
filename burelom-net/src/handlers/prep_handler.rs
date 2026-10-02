use core::cell::RefCell;

use crate::proto;
use crate::routing_table::RoutingTable;
use alloc::sync::Arc;
use hashbrown::HashMap;
use crate::logging::{info, warn};
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use crate::packetizer::Packetizer;
use crate::traits::mac::Mac;
use crate::burelom_node::{BurelomNode};
use crate::prelude::*;

/// Prep packets handler
pub struct PrepHandler {
  addr: u32,
  packetizer: Arc<Packetizer>,
  routing_table: Arc<RoutingTable>,
  mac:Arc<Box<dyn Mac>>,
  preq_awaiters: Arc<Mutex<CriticalSectionRawMutex, RefCell<HashMap<u32, async_oneshot::Sender<u32>>>>>
}

impl PrepHandler {
  pub fn new(node: &BurelomNode) -> Self {
    PrepHandler {
        addr: node.addr,
        packetizer: node.packetizer.clone(),
        routing_table: node.routing_table.clone(),
        mac: node.mac.clone(),
        preq_awaiters: node.preq_awaiters.clone(),
    }
  }

  /// Handles packets of type 'prep'. Basic logic. If packet determines for this node,
  /// complete routing request awaiter with result of operation. Else, forward packet
  /// to tho source throught gatway was early punched be preq.
  /// 
  /// #Arguments
  /// `packet` - processed packet
  pub async fn handle(&self, packet: &mut proto::Packet, gateway: u32) {
    let Some(proto::packet::Body::Prep(prep)) = packet.body.as_mut() else {
        unreachable!("handle called for non-Prep packet");
    };

    // Check if this prep for as
    if prep.source == self.addr {
      info!("<{}> Received target prep packet from={}", self.addr, prep.dest);
      // Find route request awaiter and complete them
      self.preq_awaiters.lock(|guard| {
        let mut preq_awaiters = guard.borrow_mut();
        if let Some(awaiter) = preq_awaiters.get_mut(&prep.dest) {
          awaiter.send(gateway).unwrap();
          preq_awaiters.remove(&gateway);
        } else {
          warn!("<{}> Not found preq awaiter for dest={}", self.addr, prep.dest);
        }
      });
      
    } else {
      // Else transmit prep to dest
      let route = self.routing_table.find(prep.source);

      if let Some(gw) = route {
        info!("<{}> Received transit prep packet from={}, packet send to gw={}",
          self.addr, prep.dest, gw);
        packet.hops = packet.hops + 1;
        let (_, bin) = self.packetizer.encode_packet(packet);
        self.mac.send(gw, &bin).await;
      } else {
        warn!("<{}> Not found gateway to prep from={} send", self.addr, prep.dest);
      }
    }
  }
}