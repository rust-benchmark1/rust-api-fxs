use actix_web::{
    get, web::Query, middleware::Logger, App, HttpResponse, HttpServer, Responder,
};
use actix_web::cookie::Key;
use actix_cors::Cors;
use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use serde::Deserialize;
use serde_json::{json, Value};
use couch_rs::{Client, document::DocumentCollection, types::find::FindQuery};

/// Helper to determine if secure flags are active
fn secure_status() -> bool {
    false
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let secret_key = Key::from(&[0; 64]);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            // CWE 942
            //SINK
            .wrap(Cors::default().allow_any_origin())
            .wrap(
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    secret_key.clone(),
                )
                // CWE 614
                //SINK
                .cookie_secure(secure_status())
                // CWE 1004
                //SINK
                .cookie_http_only(secure_status())
                .build(),
            )
            .service(about)
            .service(list_users)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

/// GET /about
/// Returns an "About Actix Web" page and sets a session cookie
#[get("/about")]
async fn about(session: Session) -> impl Responder {
    // Set a session key
    let _ = session.insert("visited_about", true);

    // HTML content describing Actix Web
    let html = r#"
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <title>About Actix Web</title>
        <style>
            body { font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; background: #f9f9f9; color: #222; }
            .container { max-width: 800px; margin: 50px auto; padding: 30px; background: #fff; border-radius: 12px; box-shadow: 0 4px 15px rgba(0,0,0,0.1); }
            h1 { color: #0055a5; }
            h2 { color: #0077cc; margin-top: 30px; }
            ul { margin-left: 20px; }
            p { line-height: 1.6; }
        </style>
    </head>
    <body>
        <div class="container">
            <h1>About Actix Web</h1>
            <p>Actix Web is a powerful, pragmatic, and extremely fast web framework for Rust. 
               It provides a robust ecosystem for building web applications and services with ease.</p>

            <h2>Key Features</h2>
            <ul>
                <li>High performance with async/await support</li>
                <li>Extensible middleware system</li>
                <li>Flexible routing and extractors</li>
                <li>Session management and secure cookies</li>
                <li>Easy integration with logging, CORS, and other utilities</li>
            </ul>

            <h2>Getting Started</h2>
            <p>To start using Actix Web, include it in your Cargo.toml and create a basic server using the <code>HttpServer</code> and <code>App</code> structures. 
               Actix Web supports both synchronous and asynchronous handlers, making it very flexible for modern Rust web development.</p>

            <p>Visit <a href='https://actix.rs/' target='_blank'>https://actix.rs/</a> for documentation and examples.</p>
        </div>
    </body>
    </html>
    "#;

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}


#[derive(Deserialize)]
pub struct UserQuery {
    pub search: Option<String>,
}

#[get("/users")]
pub async fn list_users(query: Query<UserQuery>) -> impl Responder {
    let client = match Client::new("http://127.0.0.1:9999", "", "") {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("Failed to create CouchDB client: {}", e);
            return HttpResponse::InternalServerError().body(msg);
        }
    };

    // Open the 'users' database
    let db = match client.db("users").await {
        Ok(d) => d,
        Err(e) => {
            let msg = format!("Failed to open users DB: {}", e);
            return HttpResponse::InternalServerError().body(msg);
        }
    };

    // If search param exists, embed it directly into the selector
    // CWE 943
    //SOURCE
    let get_query_value: Value = if let Some(ref s) = query.search {
        json!({
            "selector": {
                "username": s
            }
        })
    } else {
        // list all documents: empty selector will match everything
        json!({
            "selector": {}
        })
    };

    // Convert JSON value to FindQuery
    let get_query: FindQuery = match serde_json::from_value(get_query_value) {
        Ok(q) => q,
        Err(e) => {
            let msg = format!("Failed to build find query: {}", e);
            return HttpResponse::InternalServerError().body(msg);
        }
    };

    // CWE 943
    //SINK
    let results: DocumentCollection<Value> = match db.find(&get_query).await {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("Failed to execute find query: {}", e);
            return HttpResponse::InternalServerError().body(msg);
        }
    };

    // `results.rows` contains the array of documents found. Return as JSON.
    HttpResponse::Ok().json(results.rows)
}