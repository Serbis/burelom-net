use alloc::sync::Arc;
use crate::logging::info;
use xutex::Mutex;
use alloc::vec::Vec;
#[cfg(feature = "tokio")]
use tokio::time::Instant;
#[cfg(feature = "embassy")]
use embassy_time::Instant;

use crate::{node_registry::{IdentyType, NodeRegistry}, proto};

/// Routing table entry, descibes route to an unique node
#[derive(Debug, Clone)]
pub struct RoutinRow {
  /// Address of node to wich route leads
  pub target: u32,
  /// Node to which need to send packet to advance tranmission
  pub gateway: u32,
  /// Count of intermediate nodes between us and target
  pub hops: u32,
  /// Timestamp of last route information update
  pub last_seen: Instant
}

/// Routing table of the node
pub struct RoutingTable {
  // Self node address
  addr: u32,
  // Known node registry
  node_registry: Arc<NodeRegistry>,
  // Table
  pub routing_table: Arc<Mutex<Vec<RoutinRow>>>,
}

impl RoutingTable {
  pub fn new(addr: u32, node_registry: Arc<NodeRegistry>) -> Self {
    RoutingTable {
      addr,
      node_registry,
      routing_table: Arc::new(Mutex::new(vec![]))
    }
  }

  /// Update routing table based on incoming packet header. If talbe has no route entry,
  /// it will be create. If it is exist, last_seen will be updated. If current route
  /// to target has less hops than route in table, gateway of entry will be replaced by
  /// current and hops count will by updated in accordance of data from the packet
  /// header. Also this operation trigger update of node registry database.
  /// 
  /// # Arguments 
  /// `pakcet` - incoming packet
  /// `gatway` - gateway node where are packet from come
  pub fn update(&self, packet: &proto::Packet, gateway: u32) {
    let mut guard = self.routing_table.lock();
    let routing_table = &mut *guard;   

    match routing_table
      .iter_mut()
      .find(|v| v.target == packet.source) 
    {
      Some(route) => {
        if route.hops > packet.hops {
          route.gateway = gateway;
          route.hops = packet.hops;
          route.last_seen = Instant::now();
        } else {
          route.last_seen = Instant::now();
        }
         info!("<{}> Update route target={}, gateway={}, hops={}",
            self.addr, &route.target, &route.gateway, &route.hops);
        self.node_registry.update_from_rt(route, IdentyType::from(packet));
      }
      None => {
        if packet.source == self.addr {
          return;
        }
        let route = RoutinRow {
          target: packet.source,
          gateway: gateway,
          hops: packet.hops,
          last_seen: Instant::now()
        };
        info!("<{}> Created route target={}, gateway={}, hops={}",
        self.addr, &route.target, &route.gateway, &route.hops);
        self.node_registry.update_from_rt(&route, IdentyType::from(packet));
        routing_table.push(route);
      }
    }
  }

  /// Find gateway by destination adress
  /// 
  /// # Arguments
  /// `dest` - destination adress
  /// 
  /// # Returns - gateway or None if route not found
  pub fn find(&self, dest: u32) -> Option<u32> {
    self.routing_table
      .lock()
      .iter()
      .find(|v| v.target == dest)
      .map(|v| v.gateway)
  }
}