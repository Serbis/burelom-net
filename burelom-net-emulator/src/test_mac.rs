use std::sync::{Arc};
use async_trait::async_trait;
use burelom_net::traits::mac::Mac;
use crate::network_coordinator::{NetworkCoorditor};

pub struct TestMac {
  network_coordinator: Arc<NetworkCoorditor>,
  addr: u32
}

impl TestMac {
  pub fn new(addr: u32, network_coordinator: Arc<NetworkCoorditor>) -> Self {
    TestMac {
      network_coordinator,
      addr
    }
  }
}

#[async_trait]
impl Mac for TestMac {
  async fn send(&self, addr: u32, data: &Vec<u8>) {
    self.network_coordinator.send_from_mac(self.addr, addr, data);
  }

  async fn recv(&self) -> (u32, Vec<u8>) {
    self.network_coordinator.recv_from_mac(self.addr).await
  }
}