use alloc::vec;
use alloc::vec::{Vec};
use burelom_net::traits::cipher::Cipher;
use esp_hal::aes::cipher_modes::Ctr;
use esp_hal::aes::{AesContext, Operation};
use esp_hal::rng::Rng;
use cmac::{Cmac, Mac};
use aes::Aes128;
use anyhow::{bail, Result};
use anyhow::anyhow;

static NONCE_LEN: usize = 8;
static CMAC_LEN: usize = 4;

pub struct Esp32Cypher {
  key: [u8; 16],
  rng: Rng,  
}

impl Esp32Cypher {
  pub fn new(key: [u8; 16]) -> Self {
    let rng = Rng::new();
    
    Esp32Cypher {
      key,
      rng,
    }
  }
}

impl Cipher for Esp32Cypher {
  fn encrypt(&self, plaintext: &[u8]) -> Result<([u8; 8], Vec<u8>)> {
    let mut nonce = [0u8; NONCE_LEN];
    self.rng.read(&mut nonce);
    
    let mut iv = [0u8; 16];
    iv[..NONCE_LEN].copy_from_slice(&nonce);
    
    let mut ctx = AesContext::new(Ctr::new(iv), Operation::Encrypt, self.key);
    
    let mut ciphertext = vec![0u8; plaintext.len()];
    let handle = ctx
    .process(plaintext, &mut ciphertext)
    .map_err(|e| anyhow!("AES init failed: {:?}", e))?;
    handle.wait_blocking();
    
    let mut mac = Cmac::<Aes128>::new_from_slice(&self.key)
      .map_err(|e| anyhow!("Cmac init failed: {:?}", e))?;
    mac.update(&nonce);
    mac.update(&ciphertext);
    let tag = mac.finalize().into_bytes();
    
    let mut body = Vec::with_capacity(ciphertext.len() + CMAC_LEN);
    body.extend_from_slice(&ciphertext);
    body.extend_from_slice(&tag[..CMAC_LEN]);
    
    Ok((nonce, body))
  }
  
  fn decrypt(&self, nounce: &[u8; 8], data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < CMAC_LEN {
      bail!("Data is too short");
    }
    
    let (ciphertext, recv_mac) = data.split_at(data.len() - CMAC_LEN);
    
    let mut mac = <Cmac<Aes128> as Mac>::new_from_slice(&self.key)
      .map_err(|e| anyhow!("Cmac init failed: {:?}", e))?;
    mac.update(nounce);
    mac.update(ciphertext);
    let tag = mac.finalize().into_bytes();
    
    if tag[..CMAC_LEN] != *recv_mac {
      bail!("Wrong CMAC");
    }
    
    let mut iv = [0u8; 16];
    iv[..NONCE_LEN].copy_from_slice(nounce);
    
    let mut ctx = AesContext::new(Ctr::new(iv), Operation::Decrypt, self.key);
    
    let mut plaintext = vec![0u8; ciphertext.len()];
    let handle = ctx
    .process(ciphertext, &mut plaintext)
    .map_err(|e| anyhow!("AES init failed: {:?}", e))?;

    handle.wait_blocking();
    
    Ok(plaintext)
  }
}