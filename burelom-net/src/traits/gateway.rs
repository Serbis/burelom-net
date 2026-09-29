use async_trait::async_trait;

/// Gateway functuionality adapter. Used when device perform role GATEWAY for
/// provde interlayer between networks boundaries. Exmple implementation of 
/// this trait may be seen in test_gateway.rs of test-host sub-crate.
#[async_trait]
pub trait Gateway: Send + Sync {

  /// Send data to external network. This method called by the node, when it recives
  /// tartget datagram packet. In implemetation you must send this data to the external
  /// сonsumer. For exmample put it to mqtt queue, or trasnlate adresses, and send data
  /// to tcp layer.
  /// 
  /// # Arguments
  /// `source` - mesh network address where are data from
  /// `data` - payload
  async fn send(&self, source: u32, data: &Vec<u8>);


  /// Reveive data from external network. This methid called by node and await when data
  /// is comes. Impl must realize here receving data from other network, such as awaitng
  /// message from mqtt, or handling tcp packets.
  ///
  /// # Returns - (dest, data) - data wich must be sent to the mesh node with dest
  ///  address 
  async fn recv(&self) -> (u32, Vec<u8>);
}