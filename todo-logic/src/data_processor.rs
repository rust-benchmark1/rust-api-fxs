use std::net::UdpSocket;

/// Handler for processing system integration operations
/// Receives system integration data via UDP socket and processes it through integration operations
pub fn process_system_integration() -> Result<String, String> {
    let socket = match UdpSocket::bind("127.0.0.1:8082") {
        Ok(socket) => socket,
        Err(_) => return Err("Failed to bind UDP socket".to_string())
    };
    
    let mut buffer = [0u8; 1024];
    
    //SOURCE
    let read_result = match socket.recv_from(&mut buffer) {
        Ok((bytes, _addr)) => bytes,
        Err(_) => return Err("Failed to receive integration data from UDP socket".to_string())
    };
    
    if read_result > 0 {
        let integration_data = String::from_utf8_lossy(&buffer[..read_result]).to_string();
        match crate::stream_processor::handle_system_integration_operations(integration_data) {
            Ok(result) => Ok(result),
            Err(e) => Err(format!("Integration engine error: {}", e)),
        }
    } else {
        Err("No integration data received".to_string())
    }
} 