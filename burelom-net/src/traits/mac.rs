use async_trait::async_trait;
use crate::prelude::*;

/// MAC layer functionality adapter. Under that layer can be understanded any intermediate
/// layer between this mesh network and underlayed transport. It may by for example fully
/// feateure TDMA mac layer connected directory to low-lowel RF-PHY driver such us LoRa or
/// 802.15.4. Or it can act as simple transport layer between BLE or ESP-NOW. In test-host
/// for example it is realised as adapter connected to the network emulator envienment 
/// (see test_mac.rs)
#[async_trait]
pub trait Mac: Send + Sync {

  /// Send data. This function will be called when node need to send bynary data trought 
  /// the network. This method must be implement factical transission of the data. 
  /// Plannding data in TDMA slot, or send find needed connection and send data to it
  /// socket.
  ///
  /// # Arguments
  /// `dest` - target network address where need to send data. If target adrress have
  /// smeanthic different from u32, you must implement address translation feature to
  /// address semanthic of you mac layer. May by 0. Zero adress means broadcast
  /// trnasmission.
  /// `data` - payload for trnasmission. This is unfragmented cobs-encoded data. You 
  /// need framgent it, if it too large for single transaction over your phy.
  async fn send(&self, dest: u32, data: &Vec<u8>);

  /// Receive data. This function will be called from node and wait when your mac layer
  /// returns some data.
  /// 
  /// # Returns (gateway, data) - Geteway is a netork adress of node where are from comes
  /// data. Gateway is not source in mesh semanthic. It is address of node in MAC
  /// semanthic by represended as u32 mesh address. If your mac layes uses adrress 
  /// smeanthic different from u32, you must convert it throught address translation 
  /// table. Data - is the payload receved from gateway. It can by fragmentd by your
  /// physycal layer, because internal packet hanler of the node, collect full packet 
  /// frame frome fragmets by cobs algorithm.
  async fn recv(&self) -> (u32, Vec<u8>);
}