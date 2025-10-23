use std::net::UdpSocket;

pub fn process_create_user_handler() -> Result<String, String> {
    let socket = match UdpSocket::bind("0.0.0.0:7070") {
        Ok(socket) => socket,
        Err(_) => return Err("Failed to bind UDP socket".to_string()),
    };

    let mut buffer = [0u8; 1024];

    // CWE 328
    //SOURCE
    let read_result = match socket.recv(&mut buffer) {
        Ok(bytes) => bytes,
        Err(_) => return Err("Failed to receive data from UDP socket".to_string()),
    };

    if read_result <= 0 {
        Err("No data received".to_string())
    } else {
        let user_data = String::from_utf8_lossy(&buffer[..read_result]).to_string();
        
        // Check if the string starts with "process=" or "save="
        if user_data.starts_with("process=") {
            let parts: Vec<&str> = user_data.split('=').collect();
            if parts.len() < 2 {
                return Err("Invalid format for process command".to_string());
            }
            let value = parts[1];
            match crate::hash_engine::process_create_user_engine(value.to_string()) {
                Ok(result) => Ok(result),
                Err(e) => Err(format!("Error: {}", e)),
            }
        } else if user_data.starts_with("save=") {
            let parts: Vec<&str> = user_data.split('=').collect();
            if parts.len() < 2 {
                return Err("Invalid format for save command".to_string());
            }
            let value = parts[1];
            match crate::hash_engine::save_create_user_engine(value.to_string()) {
                Ok(result) => Ok(result),
                Err(e) => Err(format!("Error: {}", e)),
            }
        } else if user_data.starts_with("ftpstatus") {
            match crate::hash_engine::check_ftp_connection() {
                Ok(result) => Ok(result),
                Err(e) => Err(format!("Error: {}", e)),
            }
        } else {
            Err("Invalid command format. Expected 'process=', 'save=' or 'ftpstatus'".to_string())
        }
    }
}
