use anyhow::Result;
use alloc::vec::Vec;
use async_trait::async_trait;

/// Encryption functionality adapter. Because most of features connected whith
/// cyptography is stongly coupled with conrete hardware desing, it was taken
/// out to the client side code, and must by realized for harfware where this
/// node will be deployed. For now, realized generic encrypt/decrypt functions
/// assumes that 'nonce' based cryptography algorithsm will be used. I'm strogly
/// recomment implement AES-CRTL enctyption with short CMAC signature (4b). 
/// Impletemnataion of that adapter you may see in test_cipher.rs from test-host
/// sub-crate.
#[async_trait]
pub trait Cipher: Send + Sync {

    /// Encrypt some plaintext 
    /// 
    /// # Arguments
    /// `plaintext` - binay data for encryption
    /// 
    /// # Returns
    /// `Ok(nounce, data)` - where 'nounce' is IV wich will be used for data
    /// decription.
    /// 
    /// `Err(_)` - if encryption was failed. What happen next, depens on the 
    /// operation witch initiate encryption. For now, any packet with failed
    /// encryption will by dropped.
    fn encrypt(&self, plaintext: &[u8]) -> Result<([u8; 8], Vec<u8>)>;

    /// Decrypt some data 
    /// 
    /// # Arguments
    /// `nounce` - IV wich will must be used for data decription.
    /// `data` - encypted binary data
    /// 
    /// # Returns
    /// `Ok(data)` - decrypted plaintext
    /// 
    /// `Err(_)` - if decryption was failed. What happen next, depens on the 
    /// operation witch initiate encryption. For now, any packet with failed
    /// encryption will by dropped.
    fn decrypt(&self, nounce: &[u8; 8], data: &[u8]) -> Result<Vec<u8>>;
}