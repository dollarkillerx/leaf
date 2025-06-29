use anyhow::{anyhow, Result};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::Rng;
use std::sync::Arc;

#[derive(Clone)]
pub struct CryptoManager {
    cipher: Arc<Aes256Gcm>,
}

impl CryptoManager {
    pub fn new(key: &str) -> Result<Self> {
        // 解码 base64 密钥
        let key_bytes = STANDARD.decode(key)?;
        if key_bytes.len() != 32 {
            return Err(anyhow!("密钥长度必须是 32 字节"));
        }
        
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        
        Ok(Self {
            cipher: Arc::new(cipher),
        })
    }
    
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        // 生成随机 nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // 加密数据
        let ciphertext = self.cipher.encrypt(nonce, data)
            .map_err(|e| anyhow!("加密失败: {}", e))?;
        
        // 组合 nonce + ciphertext
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }
    
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 12 {
            return Err(anyhow!("数据长度不足"));
        }
        
        // 分离 nonce 和 ciphertext
        let nonce_bytes = &data[..12];
        let ciphertext = &data[12..];
        
        let nonce = Nonce::from_slice(nonce_bytes);
        
        // 解密数据
        let plaintext = self.cipher.decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("解密失败: {}", e))?;
        
        Ok(plaintext)
    }
    
    pub fn generate_key() -> String {
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill(&mut key_bytes);
        STANDARD.encode(key_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encrypt_decrypt() {
        let key = CryptoManager::generate_key();
        let crypto = CryptoManager::new(&key).unwrap();
        
        let original_data = b"Hello, World!";
        let encrypted = crypto.encrypt(original_data).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        
        assert_eq!(original_data, decrypted.as_slice());
    }
    
    #[test]
    fn test_key_generation() {
        let key1 = CryptoManager::generate_key();
        let key2 = CryptoManager::generate_key();
        
        assert_ne!(key1, key2);
        assert_eq!(key1.len(), 44); // base64 编码的 32 字节
    }
} 