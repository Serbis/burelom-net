use crate::logging::{info, warn};
use crate::{
  packetizer::{Packetizer}, proto, routing_table::RoutingTable, burelom_node::BurelomNode, traits::mac::Mac
};
use alloc::sync::Arc;
use crate::prelude::*;

/// Rerr packet handler
pub struct RerrHandler {
  addr: u32,
  packetizer: Arc<Packetizer>,
  routing_table: Arc<RoutingTable>,
  mac: Arc<Box<dyn Mac>>,
}

impl RerrHandler {
  pub fn new(node: &BurelomNode) -> Self {
    RerrHandler {
        addr: node.addr,
        packetizer: node.packetizer.clone(),
        routing_table: node.routing_table.clone(),
        mac: node.mac.clone()
    }
  }

  /// Handles packets of type 'rerr'. Basic logic. Any rerr packet always 
  /// invalidates routes to original_dest through gateway where are rerr 
  /// comes from. If the not is not last element in rerr route, it will
  /// transmit to the next hop.
  ///
  /// #Arguments
  /// `from_gateway` - gateway where are packet from come
  /// `packet` - processed packet
  pub async fn handle(&self, from_gateway: u32, packet: &mut proto::Packet) {
    let Some(proto::packet::Body::Rerr(rerr)) = packet.body.as_mut() else {
        unreachable!("handle called for non-Rerr packet");
    };

     info!("<{}> Received rerr packet original_dest={}", self.addr, rerr.original_dest);

    // Instruct rounting table that routes to original_dest through gw is now
    // died
    self.routing_table.invalidate_route(from_gateway, rerr.original_dest);

    // If the node is not last element in the route
    if packet.dest != self.addr {
      let route = self.routing_table.find(packet.dest);

      if let Some(gw) = route {
        packet.hops = packet.hops + 1;
        let (_, bin) = self.packetizer.encode_packet(packet);
        self.mac.send(gw, &bin).await;
      } else {
        warn!("<{}> Not found gateway to rerr send from={}", self.addr, packet.source);
      }
    }
  }
}