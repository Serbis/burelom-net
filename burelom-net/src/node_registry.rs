use core::cell::RefCell;
use alloc::vec;
use alloc::sync::Arc;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use crate::{proto, roles::device_roles::DeviceRole, routing_table::RoutinRow};
use enumset::{EnumSet, EnumSetType};
#[cfg(feature = "tokio")]
use tokio::time::Instant;
#[cfg(feature = "embassy")]
use embassy_time::Instant;
use crate::prelude::*;

#[derive(Debug, EnumSetType)]
pub enum IdentyType {
  PREQ, PREP, DATAGRAM, HELLO, UNKNOWN
}

/// Helper for detecting identity from network packet
impl From<&proto::Packet> for IdentyType {
    fn from(value: &proto::Packet) -> Self {
        match value.body {
          Some(proto::packet::Body::Preq(_)) => IdentyType::PREQ,
          Some(proto::packet::Body::Prep(_)) => IdentyType::PREP,
          Some(proto::packet::Body::Datagram(_)) => IdentyType::DATAGRAM,
          Some(proto::packet::Body::Hello(_)) => IdentyType::HELLO,
          None => IdentyType::UNKNOWN,
        }
    }
}

/// Description of network participant. This information collected by time of network 
/// working and periodically may not complete. Optional field may absense in some
/// moments of network lifecycle.
#[derive(Clone, Debug)]
pub struct NodeRegistryRow {
  /// Node netork address
  pub addr: u32,
  /// Node textual name
  pub name: Option<String>,
  /// Count of hops wich need overcoming by packet before it will reach the participant
  pub hops: u32,
  /// When the node last seen any packet from participant (including transit)
  pub last_seen: Instant,
  /// Addresses of nearest nodes around participant
  pub neighbour_nodes: Option<Vec<u32>>,
  /// Source of last row update
  pub identy_type: EnumSet<IdentyType>,
  /// Roles of the participant
  pub roles: Option<EnumSet<DeviceRole>>
}

/// Register of nodes known by the node. That database collected information about active
/// network participants from routing table and transit hello packets and by fact
/// presents - what the node know about the netwok - topolopy, participats,  their roles, 
/// distanse, when their be last alive and other usefull informaion which can by used by
/// client code.
pub struct NodeRegistry {
  pub table: Arc<Mutex<CriticalSectionRawMutex, RefCell<Vec<NodeRegistryRow>>>>
}

impl NodeRegistry {
  pub fn new() -> Self {
    NodeRegistry {
        table: Arc::new(Mutex::new(RefCell::new(vec![])))
    }
  }

  /// Update registry from data presented by routing table. This function calls everytume
  /// when routing table updates. It update information specific for routing mechanism -
  /// addr, hops, last_seen and etc.
  /// 
  /// # Arguments
  /// `route_entry` - routing table entry
  /// `identy_type` - indentity type basen on the packet type than was trigger update
  pub fn update_from_rt(&self, route_entry: &RoutinRow, identy_type: IdentyType) {
    self.table.lock(|guard| {
      let mut table = guard.borrow_mut();
      let exist = table
        .iter_mut()
        .find(|v| v.addr == route_entry.target);
        
      if let Some(exist) = exist {
        exist.hops = route_entry.hops;
        exist.identy_type.insert(identy_type);
        exist.last_seen = route_entry.last_seen;
      } else {
        let row = NodeRegistryRow {
          addr: route_entry.target,
          name: None,
          hops: route_entry.hops,
          last_seen: route_entry.last_seen,
          neighbour_nodes: None,
          identy_type: identy_type.into(),
          roles: None
        };

        table.push(row);
      }
    });
    
  }

  /// Update registry data from hello packet handler. This function call everytime when
  /// the node handles transit hello packet. Because at this stage entry always exist 
  /// (it cretas by routing table), this funstion updates only fields sepcified for hello
  /// packets - name, roles, neighbor and etc. Identity is alway HELLO.
  /// 
  /// # Arguments
  /// `hello` - body of handled hello packet
  /// `addr` - address of source node
  /// `hops` - updated because hello hops may be deffenet from routing hops
  pub fn update_from_hello(&self, hello: &proto::Hello, addr: u32, hops: u32) {
    self.table.lock(|guard| {
      let mut table = guard.borrow_mut();
      let exist = table
        .iter_mut()
        .find(|v| v.addr == addr);
        
      if let Some(exist) = exist {
        exist.addr = addr;
        exist.hops = hops;
        exist.name = Some(hello.name.clone());
        exist.roles = Some(EnumSet::<DeviceRole>::from_u32(hello.roles));
        exist.last_seen = Instant::now();
        exist.neighbour_nodes = Some(hello.neighbour.clone());
        exist.identy_type.insert(IdentyType::HELLO);
      } else {
        let row = NodeRegistryRow {
          addr,
          name: Some(hello.name.clone()),
          hops,
          last_seen: Instant::now(),
          neighbour_nodes: None,
          identy_type: IdentyType::HELLO.into(),
          roles: Some(EnumSet::<DeviceRole>::from_u32(hello.roles))
        };

        table.push(row);
      }
    });
    
  }
  
  /// Get nearest (hops==1) participants from the node
  /// 
  /// # Returns - list of nearest nodes
  pub fn get_neighbour(&self) -> Vec<NodeRegistryRow> {
    self.table.lock(|table| {
      let filtered = table.borrow()
        .iter()
        .filter(|v| v.hops == 1)
        .map(|v| v.clone())
        .collect();
      filtered
    }) 
  }

  // Get copy of the registry
  // 
  // #Returns - copy of the intrenal registry database
  pub fn get_registry(&self) -> Vec<NodeRegistryRow> {
    self.table
      .lock(|table| table.borrow().clone())
  }
}