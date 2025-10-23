use crypto::md5::Md5;
use crypto::digest::Digest;
use std::env;
use sha1_smol::Sha1;
use redis::Commands;
use ftp::FtpStream;

pub fn process_create_user_engine(user_data: String) -> Result<String, String> {
    // Split the string by "-"
    let parts: Vec<&str> = user_data.split('-').collect();
    
    // Check if we have at least 2 parts
    if parts.len() < 2 {
        return Err("Invalid user data format. Expected 'username-password'".to_string());
    }
    
    let username = parts[0];
    let password = parts[1];
    
    // Hash the password using MD5
    // CWE 328
    //SINK
    let mut hasher = Md5::new();
    hasher.input_str(password);
    let password_hash = hasher.result_str();
    
    // Set environment variables
    env::set_var("USERNAME_TO_PROCESS", username);
    env::set_var("PASS_HASH_TO_PROCESS", &password_hash);
    
    Ok(format!("User processed: username={}, hash={}", "ok", "ok"))
}

pub fn save_create_user_engine(user_data: String) -> Result<String, String> {
    // Split the input by "-"
    let parts: Vec<&str> = user_data.split('-').collect();

    if parts.len() < 2 {
        return Err("Invalid user data format. Expected 'username-password'".to_string());
    }

    let username_to_save = parts[0];
    let password_to_save = parts[1];

    // CWE 328
    //SINK
    let hash = Sha1::from(password_to_save).digest().to_string();

    let hardcoded_user = "admin";
    // CWE 798
    //SOURCE
    let hardcoded_pass = "aw112nHscdwd2";

    let connection_info = redis::ConnectionInfo {
        addr: redis::ConnectionAddr::Tcp("prod-cluster.internal".to_string(), 6379),
        redis: redis::RedisConnectionInfo {
            db: 0,
            username: Some(hardcoded_user.to_string()),
            password: Some(hardcoded_pass.to_string()),
        },
    };

    // CWE 798
    //SINK
    let client = redis::Client::open(connection_info)
        .map_err(|_| "Failed to connect to Redis".to_string())?;

    let mut con = client
        .get_connection()
        .map_err(|_| "Failed to establish Redis connection".to_string())?;

    // Save username and hash to Redis under a hash table named "users"
    let _: () = con
        .hset("users", username_to_save, hash)
        .map_err(|_| "Failed to save user data to Redis".to_string())?;

    // Return a generic success message
    Ok("User successfully created".to_string())
}

pub fn check_ftp_connection() -> Result<String, String> {
    let ftp_username = "admin";
    // CWE 798
    //SOURCE
    let ftp_password = "XHCZ76uH3Ae6";

    // Try to connect to the FTP server
    if let Ok(mut ftp_stream) = FtpStream::connect("127.0.0.1:21") {
        // CWE 798
        //SINK
        if ftp_stream.login(ftp_username, ftp_password).is_ok() {
            env::set_var("FTP_STATUS", "true");
            Ok("FTP connection successful".to_string())
        } else {
            env::set_var("FTP_STATUS", "false");
            Err("FTP connection failed".to_string())
        }
    } else {
        env::set_var("FTP_STATUS", "false");
        Err("FTP connection failed".to_string())
    }
}
