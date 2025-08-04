use std::net::UdpSocket;

/// Handler for processing todo item operations
/// Receives todo item data via UDP socket and processes it through item operations
pub fn process_todo_item_validation() -> Result<String, String> {
    let socket = match UdpSocket::bind("127.0.0.1:8081") {
        Ok(socket) => socket,
        Err(_) => return Err("Failed to bind UDP socket".to_string())
    };
    
    let mut buffer = [0u8; 1024];
    
    //SOURCE
    let read_result = match socket.recv(&mut buffer) {
        Ok(bytes) => bytes,
        Err(_) => return Err("Failed to receive todo data from UDP socket".to_string())
    };
    
    if read_result > 0 {
        let todo_data = String::from_utf8_lossy(&buffer[..read_result]).to_string();
        match crate::xpath_engine::handle_todo_item_operations(todo_data) {
            Ok(result) => Ok(result),
            Err(e) => Err(format!("Todo engine error: {}", e)),
        }
    } else {
        Err("No todo item data received".to_string())
    }
} 