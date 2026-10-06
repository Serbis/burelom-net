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
  PREQ, PREP, DATAGRAM, BEACON, HELLO, RERR, UNKNOWN
}

/// Helper for detecting identity from network packet
impl From<&proto::Packet> for IdentyType {
    fn from(value: &proto::Packet) -> Self {
        match value.body {
          Some(proto::packet::Body::Preq(_)) => IdentyType::PREQ,
          Some(proto::packet::Body::Prep(_)) => IdentyType::PREP,
          Some(proto::packet::Body::Datagram(_)) => IdentyType::DATAGRAM,
          Some(proto::packet::Body::Beacon(_)) => IdentyType::BEACON,
          Some(proto::packet::Body::Hello(_)) => IdentyType::HELLO,
          Some(proto::packet::Body::Rerr(_)) => IdentyType::RERR,
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
  pub neighbour_nodes: Option<Vec<NeighbourNode>>,
  /// Source of last row update
  pub identy_type: EnumSet<IdentyType>,
  /// Roles of the participant
  pub roles: Option<EnumSet<DeviceRole>>,
  /// Strange of RF siganl, this fild filled only when hops == 1 (direct rf visibility)
  pub rssi: Option<i32>
}

/// Desription of neighbour node
#[derive(Clone, Debug)]
pub struct NeighbourNode {
  /// Neighbour node network address
  pub addr: u32,
  /// Strange of RF signal
  pub rssi: i32
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
          roles: None,
          rssi: None
        };

        table.push(row);
      }
    });
    
  }

  /// Update registry data from hello packet handler. This function call everytime when
  /// the node handles transit hello packet. Because at this stage entry always exist 
  /// (it cretas by routing table), this function updates only fields sepcified for hello
  /// packets - name, roles, neighbor and etc. Identity is alway BEACON.
  /// 
  /// # Arguments
  /// `beacon` - body of handled beacon packet
  /// `addr` - address of source node
  /// `hops` - updated because hello hops may be deffenet from routing hops
  pub fn update_from_beacon(&self, beacon: &proto::Beacon, addr: u32, hops: u32) {
    self.table.lock(|guard| {
      let mut table = guard.borrow_mut();
      let exist = table
        .iter_mut()
        .find(|v| v.addr == addr);
        
      let rmn = beacon.neighbour
        .iter()
        .map(|v| {
          NeighbourNode {
            addr: v.addr,
            rssi: v.rssi
          }
        })
        .collect();

      if let Some(exist) = exist {
        exist.addr = addr;
        exist.hops = hops;
        exist.name = Some(beacon.name.clone());
        exist.roles = Some(EnumSet::<DeviceRole>::from_u32(beacon.roles));
        exist.last_seen = Instant::now();
        exist.neighbour_nodes = Some(rmn);
        exist.identy_type.insert(IdentyType::BEACON);
      } else {
        let row = NodeRegistryRow {
          addr,
          name: Some(beacon.name.clone()),
          hops,
          last_seen: Instant::now(),
          neighbour_nodes: Some(rmn),
          identy_type: IdentyType::BEACON.into(),
          roles: Some(EnumSet::<DeviceRole>::from_u32(beacon.roles)),
          rssi: None
        };

        table.push(row);
      }
    });
  }

  /// Update registry data from hello packet handler. This function call everytime when
  /// the node handles transit hello packet. Because at this stage entry always exist 
  /// (it cretas by routing table), this function updates only fields sepcified for hello
  /// packets - rssi. Identity is alway HELLO.
  /// 
  /// #Arguments
  /// `gateway` - source addr
  /// `rssi` - rf link strange
  pub fn update_from_hello(&self, gateway: u32, rssi: i32) {
    self.table.lock(|guard| {
      let mut table = guard.borrow_mut();
      let exist = table
        .iter_mut()
        .find(|v| v.addr == gateway);

      if let Some(exist) = exist {
        exist.rssi = Some(rssi);
        exist.identy_type.insert(IdentyType::HELLO);
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