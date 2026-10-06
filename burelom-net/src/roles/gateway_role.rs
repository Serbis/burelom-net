use alloc::sync::Arc;
use async_channel::Receiver;
use crate::logging::info;
#[cfg(feature = "tokio")]
use tokio::select;
use crate::{action_api::ActionApi, burelom_node::BurelomNode, traits::{gateway::Gateway}};
#[cfg(feature = "embassy")]
use embassy_futures::select::{select, Either};
use crate::prelude::*;

/// Gateway role task. This task do two actions. First it wait datagram data from default
/// role task, if device configured as GATEWAY. Default task send only stipped and 
/// decoded datagram payload, and source address of node where are datagram from recived.
/// Second, task wait for 'from outisde' data from client gateway. After recived that data
/// it will be inject to the network as regular datagram send opetion.
/// 
/// # Arguments
/// `gateway` - external gatway adapter provided by client code
/// `gateway_receiver` - channel for data from defalt role task 
/// `action_api` - used for inject data to the network through regular 'send' action
async fn task_body(
    gateway: Arc<dyn Gateway>,
    gateway_receiver: Arc<Receiver<(u32, Vec<u8>)>>,
    action_api: Arc<ActionApi>
) {
    info!("Applied device role GATEWAY");
    
    loop {
      // Wait on of - datagram from inner network, datagram from outer network
      #[cfg(feature = "tokio")]
      select! {
        (dest, bin) = gateway.recv() => {
          // Datagram from outside network, will be inject to the mesh
          let _ = action_api.send(dest, &bin).await;
        }
        Ok((source, bin)) = gateway_receiver.recv() => {
          // Datagram to outside network, will be sent to gateway
          gateway.send(source, &bin).await;
        }
      }

      #[cfg(feature = "embassy")]
      match select(gateway.recv(), gateway_receiver.recv()).await {
        Either::First((dest, bin)) => {
          // Datagram from outside network, will be inject to the mesh
          let _ = action_api.send(dest, &bin).await;
        }
        Either::Second(Ok((source, bin))) => {
          // Datagram to outside network, will be sent to gateway
          gateway.send(source, &bin).await;
        }
        Either::Second(Err(_)) => {
            
        }
      }
    }
}

#[cfg(feature = "embassy")]
#[embassy_executor::task]
async fn embassy_task(
  gateway: Arc<dyn Gateway>,
  gateway_receiver: Arc<Receiver<(u32, Vec<u8>)>>,
  action_api: Arc<ActionApi>
) {
  task_body(gateway, gateway_receiver, action_api).await
}

#[cfg(feature = "tokio")]
pub fn run(node: &BurelomNode) {
  let fut = task_body(
    node.gateway.as_ref().unwrap().clone(),
    node.gateway_receiver.clone(),
    node.action_api.as_ref().unwrap().clone()
  );
  tokio::spawn(fut);
}

#[cfg(feature = "embassy")]
pub fn run(node: &BurelomNode, spawner: &embassy_executor::Spawner) {
  spawner
    .spawn(embassy_task(
      node.gateway.as_ref().unwrap().clone(),
      node.gateway_receiver.clone(),
      node.action_api.as_ref().unwrap().clone()
    ).unwrap())
}