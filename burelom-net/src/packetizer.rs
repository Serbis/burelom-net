use crate::proto::{self};
use alloc::vec;
use crate::traits::cipher::Cipher;
use crate::{cobs::{self}};
use alloc::vec::Vec;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use crate::logging::{error, warn};
use alloc::sync::Arc;
use prost::Message;
use core::cell::RefCell;
use core::sync::atomic::{AtomicU32, Ordering};
use crate::prelude::*;

pub(crate) const NULL_NOUNCE: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];

/// Instrument indented for fully encapsulate binary and protobuf logic of network
/// packets
pub struct Packetizer {
  // Pacekt construcion buffer (see unsrip doc)
  buffer: Arc<Mutex<CriticalSectionRawMutex, RefCell<Vec<u8>>>>, 
  // Packet id global couner
  id_counter: Arc<Mutex<CriticalSectionRawMutex, AtomicU32>>,
  // Node cryptograhic adapter
  cipher: Box<dyn Cipher>,
  // Node self network address
  addr: u32
}

impl Packetizer {
  pub fn new(id_counter: Arc<Mutex<CriticalSectionRawMutex, AtomicU32>>, addr: u32, cipher: Box<dyn Cipher>) -> Self {
    Packetizer {
      buffer: Arc::new(Mutex::new(RefCell::new(vec![]))),
      id_counter,
      addr,
      cipher
    }
  }
  
  /// Construct binary packet blobs from cobs data stream. This funcion accepts any
  /// fragments with any granulation of cobs encoded data, collects them into full
  /// frames, through use of internal buffer. Decode cobs and returns as separate 
  /// packets binary blobs
  /// 
  /// #Arguments
  /// `data` - framed cobs data
  /// 
  /// #Returns - Zero, one or more decoded binary packets
  pub fn unstrip(&self, data: &[u8]) -> Vec<Vec<u8>> {
    let mut output: Vec<Vec<u8>> = vec![];
    let mut last_dp = 0;
    self.buffer.lock(|guard| {
      let mut buffer = guard.borrow_mut();

      for (index, &value) in data.iter().enumerate() {
        if value == 0x00 {
          buffer.extend_from_slice(&data[last_dp..index + 1]);
          
          if let Some(packet) = cobs::decode(&buffer) {
            output.push(packet);
          } else {
            warn!("Untable to decode cobs data:\n{}", const_hex::encode_upper(&*buffer));
          }
          
          *buffer = vec![];
          last_dp = index + 1;
        }
      }
      
      if last_dp < data.len() {
        buffer.extend_from_slice(&data[last_dp..]);
      }
    });
    
    
    return output;
  }
  
  /// Encode some data with 0x00 terminated cobs
  ///
  /// #Arguments
  /// `data` - payload for encodinf
  /// 
  /// #Returns - encoded data
  pub fn strip(&self, data: &[u8]) -> Vec<u8> {
    cobs::encode(data)
  }
  
  /// Helper for constructing protobuf preq packet
  ///
  /// # Arguments
  /// `dest` - address of node route to wich we try to found 
  /// 
  /// # Returns - protobuf packet with preq semanthic
  pub fn construct_preq(&self, dest: u32) -> proto::Packet {
    let preq = proto::Preq {
      hops: 0,
      ttl: 10,
      dest: dest,
      source: self.addr
    };

    self.construct_packet(0, proto::packet::Body::Preq(preq))
  }
  
  /// Helper for constructing protobuf prep packet
  ///
  /// # Arguments
  /// `preq` - preq packet wich will be reversed to preq
  /// 
  /// # Returns - protobuf packet with prep semanthic
  pub fn construct_prep(&self, preq: &proto::Preq) -> proto::Packet {
    let prep = proto::Prep {
      hops: preq.hops,
      dest: preq.dest,
      source: preq.source
    };

    self.construct_packet(preq.source, proto::packet::Body::Prep(prep))
  }
  
  /// Helper for constructing protobuf datagram packet
  ///
  /// # Arguments
  /// `dest` - address of node wich must consume create datagram
  /// `data` - valubale payload, will be encrypted
  /// 
  /// # Returns - protobuf packet with datagram semanthic
  pub fn construct_datagram(&self, data: &[u8], dest: u32) -> proto::Packet {
    let (nounce, enc) = self.encrypt_payload(data);
    let datagram = proto::Datagram {
      nounce: nounce.to_vec(),
      data: enc
    };
    self.construct_packet(dest, proto::packet::Body::Datagram(datagram))
  }

  /// Helper for constructing protobuf hello packet
  ///
  /// # Arguments
  /// `name` - name of the node
  /// `roles` - roles of the node
  /// `neighbour`  - neighbour of the node
  /// 
  /// # Returns - protobuf packet with hello semanthic
  pub fn construct_hello(&self, name: &String, roles: u32, neighbour: &Vec<u32>) -> proto::Packet {
    let hello = proto::Hello {
      name: name.clone(),
      roles,
      neighbour: neighbour.clone()
    };
    self.construct_packet(0, proto::packet::Body::Hello(hello))
  }
  
  /// Encode protobuf packet object to binary blob with cobs
  /// 
  /// # Arguments
  /// `packet` - object for encoding
  /// 
  /// # Returns - pacekt binary blob with cops stripping
  pub fn encode_packet(&self, packet: &proto::Packet) -> (u32, Vec<u8>) {
    let mut buf = Vec::new();
    packet.encode(&mut buf).unwrap();
    buf = self.strip(&buf);
    
    (packet.id, buf)
  }


  /// Helper funcition for encryption payload of datagram. Peculiarity of the current
  /// realization consist in the fact, that wen ecryption was fail, it creates
  /// so-called null-nonse encypted datagram, wich will be guaranteed dropped at
  /// reciver side.
  /// 
  /// # Arguments 
  /// `data` - plaintext for encyption
  /// 
  /// # Returns - (nounce, encrypted) - nounce will be attached to datagram header wich
  /// will be used at recever side for decrypion
  pub fn encrypt_payload(&self, data: &[u8]) -> ([u8; 8], Vec<u8>) {
    match self.cipher.encrypt(data) {
      Ok(v) => v,
      Err(e) => {
        error!("<{}> Unable to encrypt payload: {}", e.to_string(), self.addr);
        (NULL_NOUNCE, vec![0])
      }
    }
  }

  /// Helper funcition for decryption payload of datagram. Peculiarity of the current
  /// realization consist in the fact, that wen decryption was fail, it creates
  /// 0 sized datagram, wich will be guaranteed dropped.
  /// 
  /// # Arguments 
  /// `nounce` - IV from datagram header
  /// `enc` - data for decryption
  /// 
  /// # Returns - decrypted plaintext
  pub fn decrypt_payload(&self, nounce: &[u8; 8], enc: &[u8]) -> Vec<u8> {
    match self.cipher.decrypt(nounce, enc) {
      Ok(v) => v,
      Err(e) => {
        error!("<{}> Unable to decrypt payload: {}", e.to_string(), self.addr);
        vec![0]
      }
    }
  }
  
  /// Helper function for construct packet header
  /// 
  /// # Arguments
  /// `dest` - packet destination
  /// `body` - packet body
  /// 
  /// # Returns - complete protobuf packet object
  fn construct_packet(&self, dest: u32, body: proto::packet::Body) -> proto::Packet {
    proto::Packet {
      id: self.id_counter.lock(|guard| { guard.fetch_add(1, Ordering::SeqCst) }),
      source: self.addr,
      dest: dest,
      hops: 1,
      body: Some(body)
    }
  }
}


