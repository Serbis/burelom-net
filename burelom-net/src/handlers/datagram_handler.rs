use async_channel::Sender;
use crate::logging::{error, info, warn};
use crate::{
  packetizer::{self, Packetizer}, proto, roles::device_roles::DeviceRole, routing_table::RoutingTable, burelom_node::BurelomNode, traits::mac::Mac
};
use alloc::sync::Arc;

/// Datagram packet handler
pub struct DatagramHandler {
  addr: u32,
  packetizer: Arc<Packetizer>,
  routing_table: Arc<RoutingTable>,
  mac: Arc<Box<dyn Mac>>,
  datagram_sender: Arc<Sender<(u32, Vec<u8>)>>,
  gateway_sender: Arc<Sender<(u32, Vec<u8>)>>,
  is_gateway: bool
}

impl DatagramHandler {
  pub fn new(node: &BurelomNode) -> Self {
    DatagramHandler {
        addr: node.addr,
        packetizer: node.packetizer.clone(),
        routing_table: node.routing_table.clone(),
        mac: node.mac.clone(),
        datagram_sender: node.datagram_sender.clone(),
        gateway_sender: node.gateway_sender.clone(),
        is_gateway: node.roles.contains(DeviceRole::GATEWAY)
    }
  }

  /// Handles packets of type 'datagram'. Basic logic. If packet is transit it will be
  /// forwaeder to next route gateway (detect from rounting table). If packet determined
  /// for this node, it will be decrypted and send to clinet call code or to gateway if
  /// node has GATEWAY role.
  ///
  /// #Arguments
  /// `packet` - processed packet
  pub async fn handle(&self, packet: &mut proto::Packet) {
    let Some(proto::packet::Body::Datagram(datagram)) = packet.body.as_mut() else {
        unreachable!("handle called for non-Datagram packet");
    };

    // If datagram for as, recv it
    if packet.dest == self.addr {
      info!("<{}> Received target datagram packet: {}",
        self.addr, const_hex::encode_upper(&datagram.data));
      
      // If null nounce, this is broken packet
      if datagram.nounce.as_slice() == packetizer::NULL_NOUNCE {
        error!("<{}> Unable to handle datagram due null nounce, packet dropped", self.addr);
        return;
      }

      // Decrypt datagram payload
      let payload = self.packetizer.decrypt_payload(
        &datagram.nounce.as_slice().try_into().unwrap(),
        &datagram.data
      );

      // If node is gateway, send datagram to gateway role hanlder  
      if self.is_gateway {
        if let Err(e) 
          = self.gateway_sender.send((packet.source, payload)).await {
            error!("<{}>Unable to handle datagram due gateway channel error: {}", self.addr, e);
          }
      } else {
        // If node is regular, send dataram to client code awaiter
        if let Err(e) 
          = self.datagram_sender.send((packet.source, payload)).await {
            error!("<{}>Unable to handle datagram due channel error: {}", self.addr, e);
          }
      }

      
    } else {
      // Else send it through rout
      let route = self.routing_table.find(packet.dest);

      if let Some(gw) = route {
        info!("<{}> Received transit datagram packet from={}, packet send to gw={}",
          self.addr, packet.dest, gw);
        packet.hops = packet.hops + 1;
        let (_, bin) = self.packetizer.encode_packet(packet);
        self.mac.send(gw, &bin).await;
      } else {
        warn!("<{}> Not found gateway to datagram send from={}", self.addr, packet.source);
      }
    }
  }
}