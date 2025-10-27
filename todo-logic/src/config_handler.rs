use std::io::{Read, Write};
use std::net::TcpListener;

/// Simple TCP server that accepts one connection, processes data, and responds.
/// Binds to 0.0.0.0 so it listens on all IPv4 interfaces.
pub fn start_single_connection_server_all_interfaces() -> std::io::Result<()> {
    // Bind to all IPv4 interfaces on port 8080
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    println!("Listening on 0.0.0.0:8080...");

    if let Ok((mut stream, addr)) = listener.accept() {
        println!("Connection established with {}", addr);

        let mut buffer = [0u8; 1024];
        // CWE 943
        // CWE 327
        //SOURCE
        let bytes_read = stream.read(&mut buffer)?;

        if bytes_read == 0 {
            println!("No data received.");
            return Ok(());
        }

        let received_data = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
        println!("Received: {}", received_data);

        // Check if the received data starts with "savenew="
        if received_data.starts_with("savenew=") {
            let parts: Vec<&str> = received_data.split('=').collect();
            if parts.len() < 2 {
                eprintln!("Invalid format for savenew command");
                let _ = stream.write_all(b"Invalid format. Expected 'savenew=<value>'");
                return Ok(());
            }
            
            let value = parts[1];
            match crate::config_engine::save_new_config(value.to_string()) {
                Ok(result) => {
                    println!("Processed result: {}", result);
                    let _ = stream.write_all(result.as_bytes());
                }
                Err(e) => {
                    eprintln!("Error processing config: {}", e);
                    let _ = stream.write_all(b"Internal error");
                }
            }
        } else if received_data.starts_with("saversa=") {
            let parts: Vec<&str> = received_data.split('=').collect();
            if parts.len() < 2 {
                eprintln!("Invalid format for saversa command");
                let _ = stream.write_all(b"Invalid format. Expected 'saversa=<value>'");
                return Ok(());
            }
            
            let value = parts[1];
            match crate::config_engine::store_access_key_rsa(value.to_string()) {
                Ok(result) => {
                    println!("Processed result: {}", result);
                    let _ = stream.write_all(result.as_bytes());
                }
                Err(e) => {
                    eprintln!("Error processing RSA key: {}", e);
                    let _ = stream.write_all(b"Internal error");
                }
            }
        } else if received_data.starts_with("saverc4=") {
            let parts: Vec<&str> = received_data.split('=').collect();
            if parts.len() < 2 {
                eprintln!("Invalid format for saverc4 command");
                let _ = stream.write_all(b"Invalid format. Expected 'saverc4=<value>'");
                return Ok(());
            }
            
            let value = parts[1];
            match crate::config_engine::rc4_store_access_key(value) {
                Ok(result) => {
                    println!("Processed result: {}", result);
                    let _ = stream.write_all(result.as_bytes());
                }
                Err(e) => {
                    eprintln!("Error processing RC4 key: {}", e);
                    let _ = stream.write_all(b"Internal error");
                }
            }
        } else {
            // If it doesn't start with "savenew=", "saversa=" or "saverc4=", return invalid command
            eprintln!("Invalid command received: {}", received_data);
            let _ = stream.write_all(b"Invalid command");
        }
    }

    println!("Connection closed.");
    Ok(())
}
