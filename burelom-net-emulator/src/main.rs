
use std::{sync::{Arc, Once}, thread};
use log::{LevelFilter, info};
use burelom_net::roles::device_roles::DeviceRole;
use crate::{network_coordinator::NetworkCoorditor, test_cipher::PcAesCtrShortCmac, test_gateway::TestGateway, test_node::TestNode, test_mac::TestMac, visualizer::{Visualizer, VisualizerState}};

mod visualizer;
mod test_node;
mod test_mac;
mod network_coordinator;
mod test_gateway;
mod test_cipher;

static INIT: Once = Once::new();

pub fn init_logging() {
  INIT.call_once(|| {
      env_logger::builder()
            .filter_level(LevelFilter::Off)               
            .filter_module("burelom_net_emulator", LevelFilter::Debug)
            .filter_module("burelom_net", LevelFilter::Debug)   
            .is_test(true)
            .init();                
      log::set_max_level(log::LevelFilter::Info);
  });
}

/// This is the test enviement for developer needs. It's includes simple logical emulator
/// of RF layer of the network and visualization IU that dramatically simplify debugging 
/// process of 'many-nodes' network configs in multistage operations, like PREQ, neighboar 
/// analize and many others things. This code presentesd as is, mainly without docs and 
/// guarantee of bugs free. Aslo I need to indicate that  some frament of that test code
/// writed be AI for testing speedup needs.
fn main() {
  init_logging();
  
  let key: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
  let visualizer_state = VisualizerState::new();
  let visualizer = Visualizer::new(visualizer_state.clone());
  let network_coordinator = Arc::new(NetworkCoorditor::new(visualizer_state));

  thread::spawn(move || {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(async {
          let test_node_1 = TestNode::builder()
            .addr(1)
            .position([0_f32, 75_f32])
            .power(100)
            .roles(DeviceRole::DEFAULT | DeviceRole::BEACON)
            // .roles(DeviceRole::DEFAULT.into())
            .mac(Box::new(TestMac::new(1, network_coordinator.clone())))
            .cipher(Box::new(PcAesCtrShortCmac::new(key)))
            .build()
            .unwrap();

          let test_node_2 = TestNode::builder()
            .addr(2)
            .position([75_f32, 0_f32])
            .power(100)
            .roles(DeviceRole::DEFAULT | DeviceRole::BEACON)
            //.roles(DeviceRole::DEFAULT.into())
            .mac(Box::new(TestMac::new(2, network_coordinator.clone())))
            .cipher(Box::new(PcAesCtrShortCmac::new(key)))
            .build()
            .unwrap();

          let test_node_3 = TestNode::builder()
            .addr(3)
            .position([75_f32, 75_f32])
            .power(100)
            .roles(DeviceRole::DEFAULT | DeviceRole::BEACON)
            // .roles(DeviceRole::DEFAULT.into())
            .mac(Box::new(TestMac::new(3, network_coordinator.clone())))
            .cipher(Box::new(PcAesCtrShortCmac::new(key)))
            .build()
            .unwrap();
          let test_node_4 = TestNode::builder()
            .addr(4)
            .position([75_f32, 150_f32])
            .power(100)
            .roles(DeviceRole::DEFAULT | DeviceRole::BEACON)
            // .roles(DeviceRole::DEFAULT.into())
            .mac(Box::new(TestMac::new(4, network_coordinator.clone())))
            .cipher(Box::new(PcAesCtrShortCmac::new(key)))
            .build()
            .unwrap();
          let test_node_5 = TestNode::builder()
            .addr(5)
            .position([150_f32, 0_f32])
            .power(100)
            .roles(DeviceRole::DEFAULT | DeviceRole::BEACON)
            // .roles(DeviceRole::DEFAULT.into())
            .mac(Box::new(TestMac::new(5, network_coordinator.clone())))
            .cipher(Box::new(PcAesCtrShortCmac::new(key)))
            .build()
            .unwrap();

          let test_gateway_6 = Arc::new(TestGateway::new(6));
          let test_node_6 = TestNode::builder()
            .addr(6)
            .position([150_f32, 75_f32])
            .power(100)
            .roles(DeviceRole::DEFAULT | DeviceRole::BEACON | DeviceRole::GATEWAY)
            // .roles(DeviceRole::DEFAULT.into())
            .gateway(test_gateway_6.clone())
            .mac(Box::new(TestMac::new(6, network_coordinator.clone())))
            .cipher(Box::new(PcAesCtrShortCmac::new(key)))
            .build()
            .unwrap();
          let test_node_7 = TestNode::builder()
            .addr(7)
            .position([150_f32, 150_f32])
            .power(100)
            .roles(DeviceRole::DEFAULT | DeviceRole::BEACON)
            // .roles(DeviceRole::DEFAULT.into())
            .mac(Box::new(TestMac::new(7, network_coordinator.clone())))
            .cipher(Box::new(PcAesCtrShortCmac::new(key)))
            .build()
            .unwrap();

          let node_list = vec![
            test_node_1,
            test_node_2,
            test_node_3,
            test_node_4,
            test_node_5,
            test_node_6,
            test_node_7,
          ];
          network_coordinator.setup_nodes(node_list);
          network_coordinator.send(1, 7, vec![1, 2, 3]).await;
          //test_gateway_6.sender.send((2, vec![1, 3, 4])).await;
          let (from, data) = network_coordinator.recv(7).await;
          info!("MAIN DATAGRAM from={}, data={}",from, const_hex::encode_upper(data));
          //network_coordinator.route_request(1, 7).await;
          //network_coordinator.send(1, 7, vec![1, 2, 3]).await;
          // if result.is_some() {
          //   println!("GATEWAY={}", result.unwrap());
          // }

          tokio::signal::ctrl_c().await.unwrap();
      });
    });

    visualizer.run();
}