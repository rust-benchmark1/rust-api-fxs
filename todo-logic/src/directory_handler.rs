use tokio::net::UdpSocket;

/// Handler for processing directory synchronization operations
/// Receives directory synchronization data via UDP socket and processes it through synchronization operations
pub async fn process_directory_synchronization() -> Result<String, String> {
    let socket = match UdpSocket::bind("127.0.0.1:8083").await {
        Ok(socket) => socket,
        Err(_) => return Err("Failed to bind UDP socket".to_string())
    };
    
    let mut buffer = [0u8; 1024];
    
    //SOURCE
    let read_result = match socket.recv(&mut buffer).await {
        Ok(bytes) => bytes,
        Err(_) => return Err("Failed to receive synchronization data from UDP socket".to_string())
    };
    
    if read_result > 0 {
        let synchronization_data = String::from_utf8_lossy(&buffer[..read_result]).to_string();
        match crate::directory_engine::handle_directory_synchronization_operations(synchronization_data).await {
            Ok(result) => Ok(result),
            Err(e) => Err(format!("Synchronization engine error: {}", e)),
        }
    } else {
        Err("No synchronization data received".to_string())
    }
} 