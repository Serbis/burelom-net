use core::cell::RefCell;
use alloc::vec;
use core::sync::atomic::{AtomicU32};
use alloc::vec::Vec;
use async_channel::{Receiver, Sender};
use enumset::EnumSet;
use hashbrown::HashMap;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use crate::action_api::ActionApi;
use crate::node_registry::{NodeRegistry, NodeRegistryRow};
use crate::roles;
use crate::roles::device_roles::DeviceRole;
use crate::routing_table::RoutingTable;
use crate::traits::cipher::Cipher;
use crate::traits::gateway::Gateway;
use crate::{packetizer::{Packetizer}, traits::mac::Mac};
use anyhow::{Result, anyhow};
use crate::logging::{error};
use bon::{bon};
use alloc::sync::Arc;
use alloc::string::String;
#[cfg(feature = "tokio")]
use core::time::Duration;
#[cfg(feature = "embassy")]
use embassy_time::Duration;
use crate::prelude::*;

/// Main object implemening the node logic
pub struct BurelomNode {
  /// Self network address
  pub(crate) addr: u32,
  /// Utils for packets binary operations
  pub(crate) packetizer: Arc<Packetizer>,
  /// List of awaiters for route response
  pub(crate) preq_awaiters: Arc<Mutex<CriticalSectionRawMutex, RefCell<HashMap<u32, async_oneshot::Sender<u32>>>>>,
  /// Queue for input datagram on client side cond waits for incoming data
  pub(crate) datagram_sender: Arc<Sender<(u32, Vec<u8>)>>,
  /// Internal queue for gateway role, user for transfetr data between datagram
  /// handler and gateway role implementor
  pub(crate) gateway_sender: Arc<Sender<(u32, Vec<u8>)>>,
  /// Beaconing interval is the node have role BEACON
  pub(crate) beacon_interval: Duration,
  /// Name of the node
  pub(crate) name: String,
  /// Roles of the node
  pub(crate) roles: EnumSet<DeviceRole>,
  /// Queue for input datagram on client side cond waits for incoming data
  datagram_receiver: Arc<Receiver<(u32, Vec<u8>)>>,
  /// Internal queue for gateway role, user for transfetr data between datagram
  /// handler and gateway role implementor
  pub(crate) gateway_receiver: Arc<Receiver<(u32, Vec<u8>)>>,
  /// Mac layer adapter
  pub(crate) mac: Arc<Box<dyn Mac>>,
  /// Gateway adapter if the node configured as GATEWAY
  pub(crate) gateway: Option<Arc<dyn Gateway>>,
  /// Registry of seen pakcets broadcast id's
  pub(crate) seen_packet: Arc<Mutex<CriticalSectionRawMutex, RefCell<Vec<(u32, u32)>>>>,
  /// Routing table
  pub routing_table: Arc<RoutingTable>,
  /// Known nodes database
  pub(crate) node_registry: Arc<NodeRegistry>,
  /// Send api wrapper
  pub(crate) action_api: Option<Arc<ActionApi>>,
}

#[bon]
impl BurelomNode {

  /// Construct new node instance
  /// 
  /// #Example
  /// ```
  /// let mac = YourMac::new();
  /// let cipher = YourCipher::new();
  /// let gateway = YourGateway::n32();
  /// 
  /// let node = BurelomNode::builder()
  ///   .addr(1)
  ///   .name("My_node_1".into())
  ///   .roles(DeviceRole::DEFAULT | DeviceRole::BEACON | DeviceRole::GATEWAY)
  ///   .mac(Box::new(cipher))
  ///   .cipher(Box::new(cipher))
  ///   .gateway(Ard::new(gateway))
  ///   .beacon_interval(Duration::from_secs(60))
  ///   .build();
  /// 
  /// if let Ok(node) = node {
  ///   node.run();
  /// }
  ///          

  /// ```
  #[builder]
  pub fn new(
    /// Mac layer adapter, see trait definition for more details
    mac: Box<dyn Mac>,
    /// Network address, any positive u32 gross than 0
    addr: u32,  
    /// Device roles, see enum doc for more details
    roles: EnumSet<DeviceRole>,
    /// Cipther adapter, see trait definition for more details
    cipher: Box<dyn Cipher>,
    /// Beaconing interval, used when BEACON role is defined, if not set,
    /// default value of 60 sec will be setup
    beacon_interval: Option<Duration>,
    /// Device name, used for node identification when BEACON role is defined, if not set
    /// default value of Node-{addr} will be setup
    name: Option<String>,
    /// Geteway layer adapter, see trait definition for more details. Mandatory
    /// paramter if GATEWAY role is defined
    gateway: Option<Arc<dyn Gateway>>,
  ) -> Result<Self> {
  
    let id_counter = Arc::new(Mutex::new(AtomicU32::new(addr * 10)));
    let packetizer = Packetizer::new(id_counter.clone(), addr, cipher);
    let node_registry = Arc::new(NodeRegistry::new());
    let (datagram_sender, datagram_receiver) 
      = async_channel::unbounded::<(u32, Vec<u8>)>();
    let (gateway_sender, gateway_receiver) 
      = async_channel::unbounded::<(u32, Vec<u8>)>();

    if let Some(name) = &name {
      if name.len() > 20 {
        return Err(anyhow!("Node name must be less than 20 chars"));
      }
    }

    if roles.contains(DeviceRole::GATEWAY) && gateway.is_none() {
      return Err(anyhow!("Geteway must be defined for device role GATEWAY"));
    }

    let mut node = BurelomNode {
      addr,
      mac: Arc::new(mac),
      gateway: gateway.map(|v| v),
      preq_awaiters: Arc::new(Mutex::new(RefCell::new(HashMap::new()))),
      packetizer: Arc::new(packetizer),
      datagram_sender: Arc::new(datagram_sender),
      datagram_receiver: Arc::new(datagram_receiver),
      gateway_sender: Arc::new(gateway_sender),
      gateway_receiver: Arc::new(gateway_receiver),
      roles: roles,
      beacon_interval: beacon_interval.unwrap_or(Duration::from_secs(60)),
      name: name.unwrap_or(format!("Node-{}", addr)),
      seen_packet: Arc::new(Mutex::new(RefCell::new(vec![]))),
      routing_table: Arc::new(RoutingTable::new(addr, node_registry.clone())),
      node_registry,
      action_api: None
    };

    node.action_api = Some(Arc::new(ActionApi::new(&node)));

    Ok(node)
  }

  /// Run main node logic. This method creates some count of inner async tasks
  /// and then returns control back
  #[cfg(feature = "tokio")]
  pub fn run(&self) {
    if !self.roles.contains(DeviceRole::DEFAULT) {
      error!("Unalbe to start, node must have DEFAULT role");
      return;
    }

    if self.roles.contains(DeviceRole::DEFAULT) { 
      roles::default_role::run(self);
    }

    if self.roles.contains(DeviceRole::BEACON) {
      roles::beacon_role::run(self);
    }

    if self.roles.contains(DeviceRole::GATEWAY) {
      roles::gateway_role::run(self);
    }
  }

  /// Run main node logic. This method creates some count of inner async tasks
  /// and then returns control back
  #[cfg(feature = "embassy")]
  pub fn run(&self, spawner: &embassy_executor::Spawner) {
    if self.roles.contains(DeviceRole::DEFAULT) {
      error!("Unalbe to start, node must have DEFAULT role");
      return;
    }

    if self.roles.contains(DeviceRole::DEFAULT) { 
      roles::default_role::run(self, spawner);
    }

    if self.roles.contains(DeviceRole::BEACON) {
      roles::beacon_role::run(self, spawner);
    }

     if self.roles.contains(DeviceRole::GATEWAY) {
      roles::gateway_role::run(self, spawner);
    }
  }

  /// Receive some data addresed to the node
  /// 
  /// # Returns Ok(source, data) - where source is network address of node
  /// where are the data is come. May produce error if an internal internal 
  /// queues or tasks was failed (this is bad, it signals that you have some
  /// trouble on harware level, insufficent stack of heap and other reasons)
  pub async fn recv(&self) -> Result<(u32, Vec<u8>)> {
    match self.datagram_receiver.recv().await {
      Ok(result) => Ok(result),
      Err(e) => {
        Err(anyhow!("Datagram recv error: {}", e))
      }
    }
  }

  /// Send data to some node in the network. This call try to find route
  /// to the data to dest node. If route does not exist, route request will be 
  /// initialized for determain gateway to the target. If it is success, data
  /// forwarder to the found gw. Else if route not found error will be
  /// returned.
  /// 
  /// # Arguments
  /// `dest` - Target node network address
  /// `data` - Payload for transmission
  /// 
  /// # Returns - Err(_) if route to the target node was not determined
  pub async fn send(&self, dest: u32, data: &Vec<u8>) -> Result<()> {
    self.action_api.as_ref().unwrap().send(dest, data).await
  }

  /// Send data broadcast. Data from this call flood the network and it will be 
  /// received on all connected nodes
  /// 
  /// # Arguments
  /// `data` - payload for send
  pub async fn send_broadcast(&self, data: &Vec<u8>) {
    self.action_api.as_ref().unwrap().send_broadcast(data).await
  }

  /// Manual run of route request. This call initiate AODV PREQ procedure and
  /// wait for first success route. If route request failed (no PREP packets)
  /// was returned from net, call complete wtih None. Route request is long 
  /// procedure and may requre up to 5 seconds of time.
  /// 
  /// # Arguments
  /// `dest` - address of node route to wich we try to find
  /// 
  /// # Returns - route gateway of None 
  pub async fn route_request(&self, dest: u32) -> Option<u32> {
    self.action_api.as_ref().unwrap().route_request(dest).await
  }

  /// Returns copy of registry node database. This is the list of all nodes in the
  /// network about this node is know. This information may be user for node discovery,
  /// selecting or reconstructing network topoly on client code side/
  /// 
  /// # Returns - node registry, warn, this operation createы fully independend copy
  /// of the regustry, make sure that you have enough haep for this action!
  pub fn get_known_nodes(&self) -> Vec<NodeRegistryRow> {
    self.node_registry.get_registry()
  }
}