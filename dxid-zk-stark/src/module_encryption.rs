use anyhow::{anyhow, Result};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use blake3::Hasher;
use rand::RngCore;

pub struct ModuleEncryption {
    master_key: [u8; 32],
}

impl ModuleEncryption {
    pub fn new() -> Result<Self> {
        // Generate a master key for key derivation
        let mut master_key = [0u8; 32];
        OsRng.fill_bytes(&mut master_key);
        
        Ok(Self { master_key })
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Derive encryption key from master key
        let key = self.derive_key("module_encryption")?;
        let cipher = Aes256Gcm::new(&Key::from_slice(&key));
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt the data
        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        
        // Combine nonce and ciphertext
        let mut result = Vec::new();
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }

    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        if encrypted_data.len() < 12 {
            return Err(anyhow!("Invalid encrypted data: too short"));
        }
        
        // Extract nonce and ciphertext
        let nonce_bytes = &encrypted_data[..12];
        let ciphertext = &encrypted_data[12..];
        
        // Derive decryption key
        let key = self.derive_key("module_encryption")?;
        let cipher = Aes256Gcm::new(&Key::from_slice(&key));
        
        // Decrypt the data
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
        
        Ok(plaintext)
    }

    fn derive_key(&self, context: &str) -> Result<[u8; 32]> {
        let mut hasher = Hasher::new();
        hasher.update(&self.master_key);
        hasher.update(context.as_bytes());
        
        let mut key = [0u8; 32];
        key.copy_from_slice(&hasher.finalize().as_bytes()[..32]);
        Ok(key)
    }
}
