use std::{sync::Arc};
use enumset::EnumSet;
use burelom_net::{roles::device_roles::DeviceRole, burelom_node::BurelomNode, traits::{cipher::Cipher, gateway::{Gateway}, mac::Mac}};
use bon::{ bon};
use anyhow::{Result, anyhow};
use tokio::sync::{Mutex, mpsc::{self, UnboundedReceiver, UnboundedSender}};


pub struct TestNode {
    pub node: BurelomNode,
    pub addr: u32,
    pub power: u32,
    pub position: [f32; 2],
    pub data_tx: UnboundedSender<(u32, Vec<u8>)>,
    pub data_rx: Arc<Mutex<UnboundedReceiver<(u32, Vec<u8>)>>>
}


#[bon]
impl TestNode {

  //
  #[builder]
    pub fn new(
        mac: Box<dyn Mac>,
        cipher: Box<dyn Cipher>,
        gateway: Option<Arc<dyn Gateway>>,
        addr: u32,
        power: u32,
        position: [f32; 2],
        roles: EnumSet<DeviceRole>
    ) -> Result<Self> {
      if addr == 0 {
          return Err(anyhow!("addr unable be empty"));
      }
      if power <= 0 {
          return Err(anyhow!("power must by positive"));
      }  

      let node = burelom_net::burelom_node::BurelomNode::builder()
        .addr(addr)
        .mac(mac)
        .roles(roles)
        .cipher(cipher)
        .maybe_gateway(gateway)
        .build();

      let (data_tx, data_rx) = mpsc::unbounded_channel::<(u32, Vec<u8>)>();

      match node {
        Ok(n) => {
          Ok(TestNode {
            data_rx: Arc::new(Mutex::new(data_rx)),
            data_tx: data_tx,
            node: n,
            addr: addr,
            power: power,
            position: position,
          })
        }
        Err(e) => {
          Err(e)
        }
      }
    }
}