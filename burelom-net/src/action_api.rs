use crate::{packetizer::Packetizer, routing_table::RoutingTable, burelom_node::BurelomNode, traits::mac::Mac};
use alloc::sync::Arc;
use anyhow::{anyhow, Result};
use hashbrown::HashMap;
use crate::logging::info;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use core::cell::RefCell;
#[cfg(feature = "tokio")]
use core::time::Duration;
#[cfg(feature = "embassy")]
use embassy_time::Duration;
use crate::prelude::*;

pub struct ActionApi {
  packetizer: Arc<Packetizer>,
  routing_table: Arc<RoutingTable>,
  mac: Arc<Box<dyn Mac>>,
  addr: u32,
  preq_awaiters: Arc<Mutex<CriticalSectionRawMutex, RefCell<HashMap<u32, async_oneshot::Sender<u32>>>>>,
  seen_packet: Arc<Mutex<CriticalSectionRawMutex, RefCell<Vec<(u32, u32)>>>>
}

impl ActionApi {
  pub fn new(node :&BurelomNode) -> Self {
    ActionApi {
      packetizer: node.packetizer.clone(),
      routing_table: node.routing_table.clone(),
      mac: node.mac.clone(),
      addr: node.addr,
      preq_awaiters: node.preq_awaiters.clone(),
      seen_packet: node.seen_packet.clone()
    }
  }

  pub async fn send(&self, dest: u32, data: &Vec<u8>) -> Result<()> {
    // Create datagram
    let datagram = self.packetizer.construct_datagram(&data, dest);
    let (_, bin) = self.packetizer.encode_packet(&datagram);

    // Find route to target in routing talbe
    let exist_route = self.routing_table.find(dest);

    if let Some(gw) = exist_route {
      info!("<{}> Send datagram dest={} gw={} data={}",
        self.addr, dest, gw, const_hex::encode_upper(&data));
      // Send packet to device mac
      self.mac
        .send(gw, &bin)
        .await;
      Ok(())
    } else {
      // Find route by route request
      if let Some(gw) = self.route_request(dest).await {
        info!("<{}> Send dataghram dest={} gw={} data={}",
          self.addr, dest, gw, const_hex::encode_upper(&data));
        // Send packet to device mac
        self.mac
          .send(gw, &bin)
          .await;
        Ok(())
      } else {
        info!("<{}> Unabel to send dataghram, route not found dest={} data={}",
          self.addr, dest, const_hex::encode_upper(&data));
        Err(anyhow!("Route not found"))
      }
    }
  }

  pub async fn send_broadcast(&self, data: &Vec<u8>) {
    info!("<{}> Send broadcast data={}", self.addr, const_hex::encode_upper(&data));

    // Create datagram
    let datagram = self.packetizer.construct_datagram(&data, 0);
    let (_, bin) = self.packetizer.encode_packet(&datagram);

    // Send packet to device mac broadcast
    self.mac
      .send(0, &bin)
      .await;
  }

  pub async fn route_request(&self, dest: u32) -> Option<u32> {
    info!("<{}> Initialized route request dest={}", self.addr, dest);

    // Create PREQ
    let preq = self.packetizer.construct_preq(dest);
    let (packet_id, bin) = self.packetizer.encode_packet(&preq);

    // Register response waiter
    let (tx, rx) = async_oneshot::oneshot::<u32>();
    self.preq_awaiters.lock(|preq_awaiters| {
      preq_awaiters.borrow_mut().insert(dest, tx);
    });

    // Put packet to seen (for break ciquit in one hop target)
    self.seen_packet.lock(|seen_packet| {
      seen_packet.borrow_mut().push((packet_id, 0));
    });

    // Send packet to device mac broadcast
    self.mac
      .send(0, &bin)
      .await;

    // Wait response with timeout
    #[cfg(feature = "tokio")]
    let result = {
      tokio::time::timeout(Duration::from_secs(5), rx).await
    };

    #[cfg(feature = "embassy")]
    let result = {
      embassy_time::with_timeout(Duration::from_secs(5), rx).await
    };
    
    match result {
      Ok(v) => {
        self.preq_awaiters.lock(|preq_awaiter| {
          preq_awaiter.borrow_mut().remove(&dest);
        });
        if let Ok(gateway) = v {
          info!("<{}> Route request completed dest={} gw={}", self.addr, dest, gateway);
          return Some(gateway);
        } else {
          info!("<{}> Route request canceled dest={}", self.addr, dest);
          return None;
        }
      },         
      Err(_) => {
        info!("<{}> Route request timeout dest={}", self.addr, dest);
        return None;
      }, 
    }
  }
}