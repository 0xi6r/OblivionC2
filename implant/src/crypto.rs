use snow::{Builder, TransportState};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

pub struct Transport {
    state: TransportState,
    stream: TcpStream,
}

pub async fn handshake(
    mut stream: TcpStream,
    server_pubkey: &[u8; 32],
) -> Result<(Transport, Vec<u8>), Box<dyn std::error::Error>> {
    let builder = Builder::new(NOISE_PATTERN.parse()?);
    let keypair = builder.generate_keypair()?;
    
    let mut handshake = builder
        .local_private_key(&keypair.private)
        .remote_public_key(server_pubkey)
        .build_initiator()?;
    
    // Send initial message
    let mut msg = vec![0u8; 65535];
    let len = handshake.write_message(&[], &mut msg)?;
    
    stream.write_u32_le(len as u32).await?;
    stream.write_all(&msg[..len]).await?;
    
    // Read response
    let resp_len = stream.read_u32_le().await? as usize;
    if resp_len > 65535 {
        return Err("Handshake response too large".into());
    }
    
    let mut resp = vec![0u8; resp_len];
    stream.read_exact(&mut resp).await?;
    
    let mut payload = vec![0u8; 65535];
    let _ = handshake.read_message(&resp, &mut payload)?;
    
    // Convert to transport mode
    let state = handshake.into_transport_mode()?;
    
    // Derive session ID from our public key
    let session_id = keypair.public;
    
    Ok((Transport { state, stream }, session_id))
}

impl Transport {
    pub async fn read_message(&mut self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let len = self.stream.read_u32_le().await? as usize;
        if len > 65535 {
            return Err("Message too large".into());
        }
        
        let mut encrypted = vec![0u8; len];
        self.stream.read_exact(&mut encrypted).await?;
        
        let mut decrypted = vec![0u8; 65535];
        let len = self.state.read_message(&encrypted, &mut decrypted)?;
        
        Ok(decrypted[..len].to_vec())
    }
    
    pub async fn write_message(&mut self, plaintext: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let mut encrypted = vec![0u8; 65535];
        let len = self.state.write_message(plaintext, &mut encrypted)?;
        
        self.stream.write_u32_le(len as u32).await?;
        self.stream.write_all(&encrypted[..len]).await?;
        
        Ok(())
    }
}