use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::logging::{info, warn};
use xutex::Mutex;
use prost::Message;
use crate::packetizer::Packetizer;
use crate::traits::mac::Mac;
use crate::proto;
use crate::routing_table::RoutingTable;
use crate::burelom_node::{BurelomNode};

/// Preq packed handler
pub struct PreqHandler {
  seen_packet: Arc<Mutex<Vec<(u32, u32)>>>,
  addr: u32,
  packetizer: Arc<Packetizer>,
  routing_table: Arc<RoutingTable>,
  mac:Arc<Box<dyn Mac>>
}

impl PreqHandler {
  pub fn new(node: &BurelomNode) -> Self {
    PreqHandler {
        seen_packet: node.seen_packet.clone(),
        addr: node.addr,
        packetizer: node.packetizer.clone(),
        routing_table: node.routing_table.clone(),
        mac: node.mac.clone(),
    }
  }

    /// Handles packets of type 'preq'. Basic logic. Preq packets may by direct or
    /// transint. If packet transit, his id will be memorized for prevet last packet
    /// reposeng, and flooded to neighbor by mac bradcasting. Else if it is the target
    /// pakcet, it converted to prep, and backwart to gateway where are from preq was
    /// recived.
    /// 
    /// #Arguments
    /// `packet` - processed packet
  pub async fn handle(&self, packet: &mut proto::Packet) {
    let Some(proto::packet::Body::Preq(preq)) = packet.body.as_mut() else {
        unreachable!("handle called for non-Preq packet");
    };

    // Check if we early seen this preq - drop it
    let seen = self.seen_packet.lock()
      .iter()
      .any(|(id, _)| packet.id == *id);
    if seen {
      info!("<{}> Received transit preq packet from={}, packet dropped due seen",
        self.addr, preq.source);
      return;
    }

    // Add preq to seen
    self.seen_packet.lock().push((packet.id, 0));

    // Check if we target of that preq
    if preq.dest == self.addr {
      info!("<{}> Received target preq packet from={}", self.addr, preq.source);
      
      let prep = self.packetizer.construct_prep(preq);
      let (_, bin) = self.packetizer.encode_packet(&prep);
  
      let route = self.routing_table.find(preq.source);
      if let Some(gw) = route {
        info!("<{}> Send prep packet to={}, gw={}", self.addr, preq.source, gw);
        self.mac.send(gw, &bin).await;
      } else {
        warn!("<{}> Not found gateway to preq response", self.addr)
      }
    } else {
      // Check if preq ttl allow rebroadcast packet
      if preq.ttl == preq.hops + 1 {
        info!("<{}> Received transit preq packet from={}, packet dropped due ttl={}",
          self.addr, preq.source, preq.ttl);
      } else {
        info!("<{}> Received transit preq packet from={}, packet rebroadcast",
          self.addr, preq.source);
        // Increment hops and broadcast packet
        preq.hops = preq.hops + 1;
        packet.hops = packet.hops + 1;
        packet.body = Some(proto::packet::Body::Preq(preq.clone()));
        let mut buf = Vec::new();
        packet.encode(&mut buf).unwrap();
        buf = self.packetizer.strip(&buf);
        self.mac.send(0, &buf).await;
      }
    }
  }
}