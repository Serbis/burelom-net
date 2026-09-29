use async_channel::{Receiver, Sender};
use async_trait::async_trait;
use log::info;
use burelom_net::traits::gateway::Gateway;

pub struct TestGateway {
  sender: Sender<(u32, Vec<u8>)>,
  receiver: Receiver<(u32, Vec<u8>)>,
  addr: u32
}

impl TestGateway {
  pub fn new(addr: u32) -> Self {
    let (sender, receiver) = async_channel::unbounded();
    TestGateway { addr, sender, receiver }
  }
}

#[async_trait]
impl Gateway for TestGateway {
  async fn send(&self, source: u32, data: &Vec<u8>) {
    info!("<{}> Gateway received datagram from source={} with data={}",
      self.addr, source, const_hex::encode_upper(data))
  }

  async fn recv(&self) -> (u32, Vec<u8>) {
    let (dest, data) = self.receiver.recv().await.unwrap();
    info!("<{}> Gateway send datagram to mesh, dest={} with data={}",
      self.addr, dest, const_hex::encode_upper(&data));
    (dest, data)
  }
}