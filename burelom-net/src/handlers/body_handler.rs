use crate::{burelom_node::BurelomNode, handlers::{beacon_handler::BeaconHandler, datagram_handler::DatagramHandler, hello_handler::HelloHandler, prep_handler::PrepHandler, preq_handler::PreqHandler, rerr_handler::RerrHandler}, proto};

/// Generic packets router
pub struct BodyHandler {
  preq_handler: PreqHandler,
  prep_handler: PrepHandler,
  datagram_handler: DatagramHandler,
  beacon_handler: BeaconHandler,
  hello_handler: HelloHandler,
  rerr_handler: RerrHandler
}

impl BodyHandler {
  pub fn new(node: &BurelomNode) -> Self {
    BodyHandler {
      prep_handler: PrepHandler::new(node),
      preq_handler: PreqHandler::new(node),
      datagram_handler: DatagramHandler::new(node),
      beacon_handler: BeaconHandler::new(node),
      hello_handler: HelloHandler::new(node),
      rerr_handler: RerrHandler::new(node)
    }
  }

  /// Determines which concrete type of body does this packet has, and route it to
  /// dedicate handler.
  /// 
  /// # Arguments 
  /// `packet` - processed packet
  /// `gateway` - gateway where from packet was received
  /// `rssi` - strange of rf signal with packet was received
  pub async fn handle(&self, packet: &mut proto::Packet, gateway: u32, rssi: i32) {
    match packet.body.as_mut() {
      Some(proto::packet::Body::Preq(_)) => {
        self.preq_handler.handle(packet).await
      },
      Some(proto::packet::Body::Prep(_)) => {
        self.prep_handler.handle(packet, gateway).await
      },
      Some(proto::packet::Body::Datagram(_)) => {
        self.datagram_handler.handle(gateway, packet).await
      },
      Some(proto::packet::Body::Beacon(_)) => {
        self.beacon_handler.handle(packet).await
      },
      Some(proto::packet::Body::Hello(_)) => {
        self.hello_handler.handle(gateway, rssi).await
      },
      Some(proto::packet::Body::Rerr(_)) => {
        self.rerr_handler.handle(gateway, packet).await
      },
      None => {},
    }
  }
}