use std::io::Read;
use std::net::TcpStream;

/// Handler for processing path operations
/// Receives path data via TCP stream and processes it through path operations
pub fn process_path_stream() -> Result<String, String> {
    let mut stream = match TcpStream::connect("127.0.0.1:8080") {
        Ok(stream) => stream,
        Err(_) => return Err("Failed to connect to TCP stream".to_string()),
    };

    let mut buffer = [0u8; 1024];

    //SOURCE
    let read_result = match stream.read(&mut buffer) {
        Ok(bytes) => bytes,
        Err(_) => return Err("Failed to read from TCP stream".to_string()),
    };

    if read_result > 0 {
        let path_data = String::from_utf8_lossy(&buffer[..read_result]).to_string();
        match crate::path_engine::handle_path_operations(path_data) {
            Ok(result) => Ok(result),
            Err(e) => Err(format!("Path engine error: {}", e)),
        }
    } else {
        Err("No path data received".to_string())
    }
}
