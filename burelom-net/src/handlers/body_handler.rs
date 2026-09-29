use crate::{handlers::{datagram_handler::DatagramHandler, hello_handler::HelloHandler, prep_handler::PrepHandler, preq_handler::PreqHandler}, proto, burelom_node::BurelomNode};

/// Generic packets router
pub struct BodyHandler {
  preq_handler: PreqHandler,
  prep_handler: PrepHandler,
  datagram_handler: DatagramHandler,
  hello_handler: HelloHandler
}

impl BodyHandler {
  pub fn new(node: &BurelomNode) -> Self {
    BodyHandler {
      prep_handler: PrepHandler::new(node),
      preq_handler: PreqHandler::new(node),
      datagram_handler: DatagramHandler::new(node),
      hello_handler: HelloHandler::new(node)
    }
  }

  /// Determines which concrete type of body does this packet has, and route it to
  /// dedicate handler.
  /// 
  /// # Arguments 
  /// `packet` - processed packet
  /// `gateway` - gateway where from packet was received
  pub async fn handle(&self, packet: &mut proto::Packet, gateway: u32) {
    match packet.body.as_mut() {
      Some(proto::packet::Body::Preq(_)) => {
        self.preq_handler.handle(packet).await
      },
      Some(proto::packet::Body::Prep(_)) => {
        self.prep_handler.handle(packet, gateway).await
      },
      Some(proto::packet::Body::Datagram(_)) => {
        self.datagram_handler.handle(packet).await
      },
      Some(proto::packet::Body::Hello(_)) => {
        self.hello_handler.handle(packet).await
      },
      None => {},
    }
  }
}