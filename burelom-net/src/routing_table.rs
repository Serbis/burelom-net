use core::cell::RefCell;
use alloc::vec;
use alloc::sync::Arc;
use crate::{logging::info};
use alloc::vec::Vec;
#[cfg(feature = "tokio")]
use tokio::time::Instant;
#[cfg(feature = "embassy")]
use embassy_time::Instant;
#[cfg(feature = "embassy")]
use embassy_time::Duration;
#[cfg(feature = "tokio")]
use core::time::Duration;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};


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
  pub routing_table: Arc<Mutex<CriticalSectionRawMutex, RefCell<Vec<RoutinRow>>>>,
  // Time to live of route, after elaspes will be removed
  route_ttl: Duration
}

impl RoutingTable {
  
  pub fn new(addr: u32, node_registry: Arc<NodeRegistry>, route_ttl: Duration,) -> Self {
     RoutingTable {
      addr,
      node_registry,
      routing_table: Arc::new(Mutex::new(RefCell::new(vec![]))),
      route_ttl
    }
  }

  #[cfg(feature = "tokio")]
  pub fn run(&self) {
    let fut = clean_task(
      self.addr,
      self.routing_table.clone(),
      self.route_ttl
    );
    tokio::spawn(fut);
  }

  #[cfg(feature = "embassy")]
  pub fn run(&self, spawner: &embassy_executor::Spawner) {
    spawner
      .spawn(clean_task_embassy(
        self.addr,
        self.routing_table.clone(),
        self.route_ttl
      ).unwrap())
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
    self.routing_table.lock(|routing_table| {   
      let mut routing_table = routing_table.borrow_mut();
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
    });
  }

  /// Find gateway by destination adress
  /// 
  /// # Arguments
  /// `dest` - destination adress
  /// 
  /// # Returns - gateway or None if route not found
  pub fn find(&self, dest: u32) -> Option<u32> {
    self.routing_table.lock(|guard| {
      guard.borrow()
      .iter()
      .find(|v| v.target == dest)
      .map(|v| v.gateway)
    }) 
  }

  /// Remove route by mathing gateway and dest
  /// 
  /// #Arguments
  /// `gateway` - gateway part of route
  /// `dest` - target part of route
  pub fn invalidate_route(&self, gateway: u32, dest: u32) {
    self.routing_table.lock(|guard| {
      let mut routing_table = guard.borrow_mut();
      routing_table.retain(|route| {
         let died = route.gateway == gateway && route.target == dest;
         if died {
            info!("<{}> Route died to invalidation, target={}, gateway={}, hops={}",
              self.addr, route.target, route.gateway, route.hops);
          }
        !died
      });
    })
  }
}

#[cfg(feature = "embassy")]
#[embassy_executor::task]
async fn clean_task_embassy(
  addr: u32,
  routing_table: Arc<Mutex<CriticalSectionRawMutex, RefCell<Vec<RoutinRow>>>>,
  route_ttl: Duration,
) {
   clean_task(addr, routing_table, route_ttl).await;
}

/// Removes all routes with link to died gateway. Died gateways is a
/// gateway thought with no packet was passed with route_ttl interval
/// 
/// #Arguments
/// `addr` - node address
/// `routing_table` - routing table itself
/// `route_ttl` - inerval wich need to pass for gateway was die
async fn clean_task(
  addr: u32,
  routing_table: Arc<Mutex<CriticalSectionRawMutex, RefCell<Vec<RoutinRow>>>>,
  route_ttl: Duration
) {
  loop {
    routing_table.lock(|guard| {
      let mut routing_table = guard.borrow_mut();

      let died_gateways: Vec<u32> = routing_table.iter()
        .filter(|route| route.last_seen.elapsed().ge(&route_ttl))
        .map(|route| route.gateway)
        .collect();

      routing_table.retain(|route| {
        let died = died_gateways.contains(&route.gateway);
        if died {
          info!("<{}> Route died to tll, target={}, gateway={}, hops={}",
            addr, route.target, route.gateway, route.hops);
        }
        !died
      });
    });

    #[cfg(feature = "embassy")]
    embassy_time::Timer::after(route_ttl).await;
    #[cfg(feature = "tokio")]
    tokio::time::sleep(route_ttl).await;
  }
}