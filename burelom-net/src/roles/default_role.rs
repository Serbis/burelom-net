use alloc::sync::Arc;
use crate::logging::info;
use crate::{handlers::body_handler::BodyHandler, packetizer::Packetizer, proto, routing_table::{RoutingTable}, burelom_node::BurelomNode, traits::mac::Mac};
use prost::Message;
use crate::prelude::*;
#[cfg(feature = "tokio")]
use core::time::Duration;
#[cfg(feature = "embassy")]
use embassy_time::Duration;

/// Default role task. This task listen mac layer adapter for incoming packet.
/// Received packet decoded and routing table updated with data from packet
/// header. After than packet route to handler for processing.
/// 
/// # Arguments
/// `mac` - external mac layer
/// `packetizer` - for hello packet decoding
/// `routing_table` - for routes updated based on packet header data
/// `body_handler` - packet handlers router
async fn receive_task_body(
    mac: Arc<Box<dyn Mac>>,
    packetizer: Arc<Packetizer>,
    routing_table: Arc<RoutingTable>,
    body_handler: BodyHandler,
) {
    info!("Applied device role DEFAULT");

    loop {
      let (gateway, rssi, bin) = mac.recv().await;

      let packets = packetizer.unstrip(&bin);
      for packet_bin in packets {
          let mut packet = proto::Packet::decode(&packet_bin[..]).unwrap();

          routing_table.update(
              &packet,
              gateway
          );

          body_handler.handle(&mut packet, gateway, rssi).await;
      }
    }
}

async fn hello_task_body(
  addr: u32,
  mac: Arc<Box<dyn Mac>>,
  packetizer: Arc<Packetizer>,
  hello_interval: Duration
) {
  loop {
    info!("<{}> Send hello data", addr);

    // Create hello packet
    let hello = packetizer.construct_hello();
    let (_, bin) = packetizer.encode_packet(&hello);

    // Send packet to device mac broadcast
    mac
      .send(0, &bin)
      .await;

    #[cfg(feature = "embassy")]
    embassy_time::Timer::after(hello_interval).await;
    #[cfg(feature = "tokio")]
    tokio::time::sleep(hello_interval).await;
  }
}

#[cfg(feature = "embassy")]
#[embassy_executor::task]
async fn receive_embassy_task(
  mac: Arc<Box<dyn Mac>>,
  packetizer: Arc<Packetizer>,
  routing_table: Arc<RoutingTable>,
  body_handler: BodyHandler,
) {
  receive_task_body(mac, packetizer, routing_table, body_handler).await
}

#[cfg(feature = "embassy")]
#[embassy_executor::task]
async fn hello_embassy_task(
  addr: u32,
  mac: Arc<Box<dyn Mac>>,
  packetizer: Arc<Packetizer>,
  hello_interval: Duration
) {
  hello_task_body(addr, mac, packetizer, hello_interval).await
}

#[cfg(feature = "tokio")]
pub fn run(node: &BurelomNode) {
  let receive_task_fut = receive_task_body(
    node.mac.clone(),
    node.packetizer.clone(),
    node.routing_table.clone(),
    BodyHandler::new(node),
  );
  let hello_task_fut = hello_task_body(
    node.addr,
    node.mac.clone(),
    node.packetizer.clone(),
    node.hello_interval
  );
  tokio::spawn(receive_task_fut);
  tokio::spawn(hello_task_fut);
}

#[cfg(feature = "embassy")]
pub fn run(node: &BurelomNode, spawner: &embassy_executor::Spawner) {
  spawner
    .spawn(receive_embassy_task(
        node.mac.clone(),
        node.packetizer.clone(),
        node.routing_table.clone(),
        BodyHandler::new(node),
    )
    .unwrap());

  spawner
    .spawn(hello_embassy_task(
        node.addr,
        node.mac.clone(),
        node.packetizer.clone(),
        node.hello_interval
    )
    .unwrap());
}






