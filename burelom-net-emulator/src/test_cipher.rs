/// ATTINTION: Parts of this code manage by AI. Be careful when
/// copying and using this code in production solutions!

use aes::Aes128;
use cmac::{Cmac, Mac};
use ctr::cipher::{KeyIvInit, StreamCipher};
use ctr::Ctr128BE;
use anyhow::{bail, Context, Result};
use rand::Rng;
use burelom_net::traits::cipher::Cipher;
use cmac::{digest::KeyInit};

static NONCE_LEN: usize = 8;
static CMAC_LEN: usize = 4;

type Aes128Ctr = Ctr128BE<Aes128>;

pub struct PcAesCtrShortCmac {
  key: [u8; 16]
}

impl PcAesCtrShortCmac {
  pub fn new(key: [u8; 16]) -> Self {
    PcAesCtrShortCmac {
      key
    }
  }
}



impl Cipher for PcAesCtrShortCmac {
  fn encrypt(&self, plaintext: &[u8]) -> Result<([u8; 8], Vec<u8>)> {
    let mut nonce = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut nonce);
    
    let mut iv = [0u8; 16];
    iv[..NONCE_LEN].copy_from_slice(&nonce);
    
    let mut ciphertext = plaintext.to_vec();
    let mut cipher = Aes128Ctr::new(&self.key.into(), &iv.into());
    cipher.apply_keystream(&mut ciphertext);
    
    let mut mac = Cmac::<Aes128>::new_from_slice(&self.key)
    .context("CMAC init failed")?;
    mac.update(&nonce);
    mac.update(&ciphertext);
    let tag = mac.finalize().into_bytes();
    
    let mut body = Vec::with_capacity(ciphertext.len() + CMAC_LEN);
    body.extend_from_slice(&ciphertext);
    body.extend_from_slice(&tag[..CMAC_LEN]);
    
    Ok((nonce, body))
  }
  
  fn decrypt(&self, nonce: &[u8; 8], packet: &[u8]) -> Result<Vec<u8>> {
    if packet.len() < CMAC_LEN {
      bail!("packet too short: {} bytes", packet.len());
    }
    
    let (ciphertext, recv_mac) = packet.split_at(packet.len() - CMAC_LEN);
    
    let mut mac = Cmac::<Aes128>::new_from_slice(&self.key)
    .context("CMAC init failed")?;
    mac.update(nonce);
    mac.update(ciphertext);
    let tag = mac.finalize().into_bytes();
    
    if tag[..CMAC_LEN] != *recv_mac {
      bail!("invalid CMAC: packet was tampered with or key mismatch");
    }
    
    // 3. AES-128-CTR расшифровка.
    let mut iv = [0u8; 16];
    iv[..NONCE_LEN].copy_from_slice(nonce);
    
    let mut plaintext = ciphertext.to_vec();
    let mut cipher = Aes128Ctr::new(&self.key.into(), &iv.into());
    cipher.apply_keystream(&mut plaintext);
    
    Ok(plaintext)
  }
}