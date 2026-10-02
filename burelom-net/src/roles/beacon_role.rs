use enumset::EnumSet;
use crate::logging::info;
use crate::{node_registry::NodeRegistry, packetizer::Packetizer, roles::device_roles::DeviceRole, burelom_node::BurelomNode, traits::mac::Mac};
#[cfg(feature = "tokio")]
use core::time::Duration;
use alloc::sync::Arc;
#[cfg(feature = "embassy")]
use embassy_time::Duration;
use crate::prelude::*;

/// Beacon role task. This task regularaly send hello packet to the neighbor by
/// MAC brodcast. Packet conains node specicif informatin like name, device roles
/// and known neighbor devices. Packet resend interval determained at node 
/// configuration stage.
/// 
/// # Arguments
/// `addr` - node address
/// `name` - node name (specified at node configuration stage)
/// `mac` - external mac layer
/// `packetizer` - for hello packet construction
/// `beacon_interval` - hello packet resend interval
/// `roles` - current node roles
/// `node_registry` - for collecting information aboun neighbor
async fn task_body(
  addr: u32,
  name: String,
  mac: Arc<Box<dyn Mac>>,
  packetizer: Arc<Packetizer>,
  beacon_interval: Duration, 
  roles: EnumSet<DeviceRole>,
  node_registry: Arc<NodeRegistry>
) {
  info!("Applied device role BEACON");

  loop {
    info!("<{}> Send hello data", addr);


    // Create roles bitmask
    let roles = roles.as_u32();

    // Get node neighbour
    let neighbour: Vec<u32> = node_registry.get_neighbour()
      .iter()
      .map(|v| v.addr)
      .collect();

    // Create datagram
    let hello = packetizer.construct_hello(&name, roles, &neighbour);
    let (_, bin) = packetizer.encode_packet(&hello);

    // Send packet to device mac broadcast
    mac
      .send(0, &bin)
      .await;

    #[cfg(feature = "embassy")]
    embassy_time::Timer::after(beacon_interval).await;
    #[cfg(feature = "tokio")]
    tokio::time::sleep(beacon_interval).await;
  }
}

#[cfg(feature = "embassy")]
#[embassy_executor::task]
async fn embassy_task(
  addr: u32,
  name: String,
  mac: Arc<Box<dyn Mac>>,
  packetizer: Arc<Packetizer>,
  beacon_interval: Duration,
  roles: EnumSet<DeviceRole>,
  node_registry: Arc<NodeRegistry>
) {
  task_body(
    addr,
    name,
    mac,
    packetizer,
    beacon_interval,
    roles, 
    node_registry
  ).await
}

#[cfg(feature = "tokio")]
pub fn run(node: &BurelomNode) {
  let fut = task_body(
    node.addr,
    node.name.clone(),
    node.mac.clone(),
    node.packetizer.clone(),
    node.beacon_interval,
    node.roles,
    node.node_registry.clone()
  );
  tokio::spawn(fut);
}

#[cfg(feature = "embassy")]
pub fn run(node: &BurelomNode, spawner: &embassy_executor::Spawner) {
  spawner
    .spawn(embassy_task(
      node.addr,
      node.name.clone(),
      node.mac.clone(),
      node.packetizer.clone(),
      node.beacon_interval,
      node.roles,
      node.node_registry.clone()
    ).unwrap())
}






