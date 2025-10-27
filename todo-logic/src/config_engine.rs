use mongodb::{
    bson::{self, doc, Bson, Document},
    Client as MongoClient,
};
use serde_json;
use tokio::runtime::Runtime;
use std::env;


use rsa::pkcs1v15::Pkcs1v15Encrypt;
use rand::rngs::OsRng;
use rsa::{RsaPrivateKey, RsaPublicKey};
use base64;

use rc4::{Rc4, KeyInit, StreamCipher};

pub fn save_new_config(config_str: String) -> Result<String, String> {
    let rt = Runtime::new().map_err(|e| format!("Failed to create runtime: {}", e))?;

    rt.block_on(async move {
        // Connect to MongoDB
        let client = MongoClient::with_uri_str("mongodb://127.0.0.1:25000")
            .await
            .map_err(|e| format!("Failed to connect to MongoDB: {}", e))?;

        let db = client.database("default_db");
        let coll: mongodb::Collection<Document> = db.collection("configs");

        // Try to interpret input as JSON (no sanitization)
        // If valid JSON, use as selector directly; otherwise, wrap in $where
        let query_value = doc! { "$where": config_str.clone() };

        // CWE 943
        //SINK
        let result = coll.find_one(query_value, None).await.map_err(|e| format!("Query failed: {}", e))?;

        if let Some(doc) = result {
            // Convert document to JSON string for storing
            let json_value = serde_json::to_string(&doc)
                .map_err(|e| format!("Failed to serialize document: {}", e))?;

            // Store in environment variable
            env::set_var("NEW_CONFIG", &json_value);
            Ok(format!("NEW_CONFIG set to: {}", json_value))
        } else {
            Err("No document found for given query".to_string())
        }
    })
}

pub fn store_access_key_rsa(plain: String) -> Result<String, String> {
    let mut rng = OsRng;

    // generate private key
    let priv_key = RsaPrivateKey::new(&mut rng, 2048)
        .map_err(|e| format!("Key generation failed: {}", e))?;

    // derive public key
    let pub_key = RsaPublicKey::from(&priv_key);

    // plaintext bytes
    let payload = plain.into_bytes();

    // CWE 327
    //SINK
    let cipher_bytes = pub_key.encrypt(&mut rng, Pkcs1v15Encrypt, &payload)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    // encode for safe storage as text
    let encoded = base64::encode(&cipher_bytes);

    // set environment variable (process-local)
    env::set_var("ACCESS_KEY", &encoded);

    Ok("ACCESS_KEY set".to_string())
}

pub fn rc4_store_access_key(password: &str) -> Result<String, String> {
    let key = b"Jk9#xPq8!Zm7";

    // create RC4 cipher with the key
    // CWE 327
    //SINK
    let mut cipher = Rc4::new(key.into());

    // prepare plaintext bytes and encrypt in-place
    let mut bytes = password.as_bytes().to_vec();
    cipher.apply_keystream(&mut bytes);

    // hex-encode for safe storage as text
    let encoded = hex::encode(&bytes);

    // store in environment variable 
    env::set_var("ACCESS_KEY", &encoded);

    Ok("ACCESS_KEY set".to_string())
}
