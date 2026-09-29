use alloc::sync::Arc;
use crate::logging::info;
use crate::{handlers::body_handler::BodyHandler, packetizer::Packetizer, proto, routing_table::{RoutingTable}, burelom_node::BurelomNode, traits::mac::Mac};
use prost::Message;

/// Default role task. This task listen mac layer adapter for incoming packet.
/// Received packet decoded and routing table updated with data from packet
/// header. After than packet route to handler for processing.
/// 
/// # Arguments
/// `mac` - external mac layer
/// `packetizer` - for hello packet decoding
/// `routing_table` - for routes updated based on packet header data
/// `body_handler` - packet handlers router
async fn task_body(
    mac: Arc<Box<dyn Mac>>,
    packetizer: Arc<Packetizer>,
    routing_table: Arc<RoutingTable>,
    body_handler: BodyHandler,
) {
    info!("Applied device role DEFAULT");

    loop {
      let (gateway, bin) = mac.recv().await;

      info!("SIZE==={}", bin.len());
      let packets = packetizer.unstrip(&bin);
      for packet_bin in packets {
          let mut packet = proto::Packet::decode(&packet_bin[..]).unwrap();

          routing_table.update(
              &packet,
              gateway
          );

          body_handler.handle(&mut packet, gateway).await;
      }
    }
}

#[cfg(feature = "embassy")]
#[embassy_executor::task]
async fn embassy_task(
  mac: Arc<Box<dyn Mac>>,
  packetizer: Arc<Packetizer>,
  routing_table: Arc<RoutingTable>,
  body_handler: BodyHandler,
) {
  task_body(mac, packetizer, routing_table, body_handler).await
}

#[cfg(feature = "tokio")]
pub fn run(node: &BurelomNode) {
  let fut = task_body(
    node.mac.clone(),
    node.packetizer.clone(),
    node.routing_table.clone(),
    BodyHandler::new(node),
  );
  tokio::spawn(fut);
}

#[cfg(feature = "embassy")]
pub fn run(node: &BurelomNode, spawner: &embassy_executor::Spawner) {
  spawner
    .spawn(embassy_task(
        node.mac.clone(),
        node.packetizer.clone(),
        node.routing_table.clone(),
        BodyHandler::new(node),
    ).unwrap())
}






