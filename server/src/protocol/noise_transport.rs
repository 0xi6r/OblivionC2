use crate::crypto::keys::{NoiseHandshakeState, TransportState};
use crate::crypto::{CryptoError, NOISE_PATTERN};
use bytes::{Bytes, BytesMut};
use snow::Builder;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const MAX_MESSAGE_SIZE: usize = 65535;

pub struct NoiseTransport {
    stream: TcpStream,
    state: TransportState,
    buffer: BytesMut,
}

impl NoiseTransport {
    pub async fn accept(
        stream: TcpStream,
        static_key: &[u8],
    ) -> crate::Result<(Self, Vec<u8>)> {
        let mut buffer = BytesMut::with_capacity(MAX_MESSAGE_SIZE * 2);
        let mut handshake = NoiseHandshakeState::new_responder(static_key)?;
        
        // Read initial handshake message
        let len = stream.read_u32_le().await? as usize;
        if len > MAX_MESSAGE_SIZE {
            return Err(crate::OblivionError::Protocol("Handshake message too large".into()));
        }
        
        buffer.resize(len, 0);
        stream.read_exact(&mut buffer).await?;
        
        // Process handshake
        let mut payload = vec![0u8; MAX_MESSAGE_SIZE];
        let payload_len = handshake.read_message(&buffer, &mut payload)?;
        let payload = payload[..payload_len].to_vec();
        
        // Send response
        let mut response = vec![0u8; MAX_MESSAGE_SIZE];
        let response_len = handshake.write_message(b"", &mut response)?;
        let response = &response[..response_len];
        
        stream.write_u32_le(response_len as u32).await?;
        stream.write_all(response).await?;
        
        // Finalize handshake
        let state = handshake.into_transport_mode()?;
        
        Ok((
            Self {
                stream,
                state,
                buffer: BytesMut::new(),
            },
            payload,
        ))
    }
    
    pub async fn read_message(&mut self) -> crate::Result<Vec<u8>> {
        // Read length
        let len = self.stream.read_u32_le().await? as usize;
        if len > MAX_MESSAGE_SIZE {
            return Err(crate::OblivionError::Protocol("Message too large".into()));
        }
        
        // Read encrypted payload
        self.buffer.resize(len, 0);
        self.stream.read_exact(&mut self.buffer).await?;
        
        // Decrypt
        let mut plaintext = vec![0u8; MAX_MESSAGE_SIZE];
        let pt_len = self.state.read_message(&self.buffer, &mut plaintext)
            .map_err(|e| crate::OblivionError::Crypto(e.to_string()))?;
        
        Ok(plaintext[..pt_len].to_vec())
    }
    
    pub async fn write_message(&mut self, plaintext: &[u8]) -> crate::Result<()> {
        if plaintext.len() > MAX_MESSAGE_SIZE - 16 { // Account for Noise overhead
            return Err(crate::OblivionError::Protocol("Message too large to encrypt".into()));
        }
        
        let mut ciphertext = vec![0u8; MAX_MESSAGE_SIZE];
        let ct_len = self.state.write_message(plaintext, &mut ciphertext)
            .map_err(|e| crate::OblivionError::Crypto(e.to_string()))?;
        
        let ciphertext = &ciphertext[..ct_len];
        
        self.stream.write_u32_le(ct_len as u32).await?;
        self.stream.write_all(ciphertext).await?;
        
        Ok(())
    }
    
    pub fn into_inner(self) -> TcpStream {
        self.stream
    }
}