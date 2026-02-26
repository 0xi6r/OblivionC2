use super::{CryptoError, Result, IDENTITY_KEY_LEN, secure_random};
use ring::signature::Ed25519KeyPair;
use std::fs;
use std::path::Path;

pub struct ServerIdentity {
    key_pair: Ed25519KeyPair,
    public_key: [u8; 32],
}

impl ServerIdentity {
    pub fn generate() -> Result<Self> {
        let mut seed = [0u8; 32];
        secure_random(&mut seed);
        
        let key_pair = Ed25519KeyPair::from_seed_unchecked(&seed)
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        
        let public_key: [u8; 32] = key_pair.public_key().as_ref().try_into()
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        
        Ok(Self { key_pair, public_key })
    }
    
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = fs::read(path)
            .map_err(|e| CryptoError::Noise(e.to_string()))?;
        
        if contents.len() != 32 {
            return Err(CryptoError::InvalidKeyLength);
        }
        
        let seed: [u8; 32] = contents.try_into().unwrap();
        let key_pair = Ed25519KeyPair::from_seed_unchecked(&seed)
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        
        let public_key: [u8; 32] = key_pair.public_key().as_ref().try_into()
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        
        Ok(Self { key_pair, public_key })
    }
    
    pub fn save_to_file(&self, path: &Path, seed: &[u8; 32]) -> Result<()> {
        fs::write(path, seed)
            .map_err(|e| CryptoError::Noise(e.to_string()))?;
        
        // Set restrictive permissions (Unix)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms).unwrap();
        }
        
        Ok(())
    }
    
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public_key
    }
    
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.key_pair.sign(message).as_ref().to_vec()
    }
}

pub struct NoiseHandshakeState {
    state: snow::HandshakeState,
}

impl NoiseHandshakeState {
    pub fn new_responder(static_key: &[u8]) -> Result<Self> {
        let builder = snow::Builder::new(super::NOISE_PATTERN.parse()
            .map_err(|e| CryptoError::Noise(e.to_string()))?);
        
        let keypair = builder.generate_keypair()
            .map_err(|e| CryptoError::Noise(e.to_string()))?;
        
        let state = builder
            .local_private_key(&keypair.private)
            .remote_public_key(static_key)
            .build_responder()
            .map_err(|e| CryptoError::Noise(e.to_string()))?;
        
        Ok(Self { state })
    }
    
    pub fn read_message(&mut self, payload: &[u8], output: &mut [u8]) -> Result<usize> {
        self.state.read_message(payload, output)
            .map_err(|e| CryptoError::Noise(e.to_string()))
    }
    
    pub fn write_message(&mut self, payload: &[u8], output: &mut [u8]) -> Result<usize> {
        self.state.write_message(payload, output)
            .map_err(|e| CryptoError::Noise(e.to_string()))
    }
    
    pub fn into_transport_mode(self) -> Result<TransportState> {
        let state = self.state.into_transport_mode()
            .map_err(|e| CryptoError::Noise(e.to_string()))?;
        Ok(TransportState { state })
    }
}

pub struct TransportState {
    state: snow::TransportState,
}

impl TransportState {
    pub fn read_message(&mut self, payload: &[u8], output: &mut [u8]) -> Result<usize> {
        self.state.read_message(payload, output)
            .map_err(|e| CryptoError::Noise(e.to_string()))
    }
    
    pub fn write_message(&mut self, payload: &[u8], output: &mut [u8]) -> Result<usize> {
        self.state.write_message(payload, output)
            .map_err(|e| CryptoError::Noise(e.to_string()))
    }
}