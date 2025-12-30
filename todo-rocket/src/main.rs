#[macro_use]
extern crate rocket;

use log::{debug, LevelFilter};
use rocket::http::Status;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::tokio::sync::RwLock;
use rocket::{uri, State};
use simplelog::{Config, SimpleLogger};
use std::sync::Arc;
use todo_logic::{IdentifyableTodoItem, Pagination, TodoItem, TodoStore, TodoStoreError, UpdateTodoItem};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs::File;
use std::io::Write;
use rhai::Engine;
use jwt_simple::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rsa::RsaPrivateKey;
use rsa::pkcs1::EncodeRsaPrivateKey;

/// Type for our shared state
/// In our sample application, we store the todo list in memory. As the state is shared
/// between concurrently running web requests, we need to make it thread-safe.
type Db = Arc<RwLock<TodoStore>>;

/// Rocket relies heavily on macros. The launch macro will generate a tokio main function for us.
#[launch]
fn rocket() -> _ {
    // Initialize logging.
    // Rocket uses the log crate (https://crates.io/crates/log) to log requests. You can use any
    // compatible logger, but for this example we'll use simplelog. Enhancements in terms
    // of more flexible logging are planned for future releases
    // (https://github.com/SergioBenitez/Rocket/issues/21).
    SimpleLogger::init(LevelFilter::Debug, Config::default()).unwrap();

    // Create shared data store
    let db = Db::default();

    rocket::build()
        // Here we mount our routes. More details about route mounting
        // at https://rocket.rs/v0.5-rc/guide/overview/#mounting.
        .mount(
            "/",
            routes![
                get_todos, 
                get_todo, 
                add_todo, 
                update_todo, 
                delete_todo, 
                persist,
                set_user_data,
                calculate_offset,
                save_data_file,
                process_offset,
                check_memory_availability,
                run_custom_code,
                refresh_token
            ],
        )
        // Register our shared state.
        // More about using shared state at https://rocket.rs/v0.5-rc/guide/state/.
        .manage(db)
}

/// Get list of todo items
///
/// Rocket implements the FromParam trait for typically used data types so they
/// can be used to extract data from the query string. Of course you can implement
/// FromParam for custom data types, too.
/// More about it at https://rocket.rs/v0.5-rc/guide/requests/#query-strings.
///
/// Also note the Responder trait (https://rocket.rs/v0.5-rc/guide/responses/#custom-responders).
/// Rocket comes with a lot of built-in responders, but you can also
/// implement the trait for your own custom types.
#[get("/todos?<offset>&<limit>")]
async fn get_todos(offset: Option<usize>, limit: Option<usize>, db: &State<Db>) -> Json<Vec<IdentifyableTodoItem>> {
    let todos = db.read().await;
    let pagination = Pagination::new(offset, limit);
    Json(todos.get_todos(pagination))
}

/// Get a single todo item
///
/// Note that Option<T> implements the Responder trait, too. This makes it really
/// simple to return a 404 if the requested item does not exist.
#[get("/todos/<id>")]
async fn get_todo(id: usize, db: &State<Db>) -> Option<Json<IdentifyableTodoItem>> {
    let todos = db.read().await;
    todos.get_todo(id).map(|item| Json(item.clone()))
}

/// Add a new todo item
///
/// Note the use of a "Request Guard" (FromRequest trait) here. Here it is used
/// to extract the JSON body of the request. You can implement your own guards, too
/// (https://rocket.rs/v0.5-rc/guide/requests/#custom-guards). Many things that you
/// would do with middlewares in other frameworks are done with request guards in Rocket.
#[post("/todos", format = "json", data = "<todo>")]
async fn add_todo(todo: Json<TodoItem>, db: &State<Db>) -> Created<Json<IdentifyableTodoItem>> {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo.0);

    // Nice detail here: The uri macro helps you to generate URIs for your routes.
    // Very useful for building the location header.
    let location = uri!("/", get_todo(todo.id));
    Created::new(location.to_string()).body(Json(todo))
}

/// Delete a todo item
///
/// Note the extraction of the id from the path.
#[delete("/todos/<id>")]
async fn delete_todo(id: usize, db: &State<Db>) -> Status {
    match db.write().await.remove_todo(id) {
        // Note that Status represents the HTTP status code
        Some(_) => Status::NoContent,
        None => Status::NotFound,
    }
}

/// Update a todo item
#[patch("/todos/<id>", format = "json", data = "<input>")]
async fn update_todo(id: usize, input: Json<UpdateTodoItem>, db: &State<Db>) -> Option<Json<IdentifyableTodoItem>> {
    let mut todos = db.write().await;
    let res = todos.update_todo(&id, input.0);
    res.map(|todo| Json(todo.clone()))
}

/// Application-level error object
///
/// Note how easy it is to implement Rocket's Responder trait with
/// the macros that Rocket provides.
#[derive(Responder)]
enum AppError {
    #[response(status = 500)]
    InternalError(String),
}
impl From<TodoStoreError> for AppError {
    fn from(inner: TodoStoreError) -> Self {
        AppError::InternalError(Json(inner).to_string())
    }
}

/// Persist the todo store to disk
#[post("/todos/persist")]
async fn persist(db: &State<Db>) -> Result<(), AppError> {
    debug!("Persisting todos");
    let todos = db.read().await;
    todos.persist().await?;
    Ok(())
}
// ============================================================================
// Validation Functions
// ============================================================================

fn validate_integer_range(value: i32) -> i32 {
    let max_threshold = 1_000_000;
    if value > max_threshold {
        debug!("Warning: Value {} exceeds threshold {}", value, max_threshold);
    }
    value
}

fn validate_positive_integer(value: i32) -> i32 {
    if value < 0 {
        debug!("Warning: Negative value detected: {}", value);
    }
    value
}

fn validate_string_length(value: String) -> String {
    let max_length = 1000;
    if value.len() > max_length {
        debug!("Warning: String length {} exceeds maximum {}", value.len(), max_length);
    }
    value
}

fn validate_safe_characters(value: String) -> String {
    let dangerous_chars = ['<', '>', '&', '"', '\'', '/', '\\'];
    for c in dangerous_chars {
        if value.contains(c) {
            debug!("Warning: Potentially dangerous character '{}' found in input", c);
            break;
        }
    }
    value
}

fn validate_memory_size(value: usize) -> usize {
    let max_memory = 1024 * 1024 * 100;
    if value > max_memory {
        debug!("Warning: Memory allocation {} exceeds limit {}", value, max_memory);
    }
    value
}

// ============================================================================
// User Routes
// ============================================================================

#[post("/setuserdata", data = "<user_data>")]
//CWE 502
//SOURCE
pub fn set_user_data(user_data: String) -> Result<String, Status> {
    let validated_length = validate_string_length(user_data);
    let validated_data = validate_safe_characters(validated_length);
    
    //CWE 502
    //SINK
    let user: Value = serde_json::from_str(&validated_data)
        .map_err(|_| Status::BadRequest)?;
        
    if let Some(pref) = user.get("preferences") {
        env::set_var("USER_PREFERENCES", pref.to_string());
    }
    Ok("User data saved successfully".to_string())
}

#[get("/calculateoffset?<divisor>")]
//CWE 369
//SOURCE
pub fn calculate_offset(divisor: i32) -> String {
    let validated_range = validate_integer_range(divisor);
    let validated_divisor = validate_positive_integer(validated_range);
    
    let base_value: i32 = 1024;
    //CWE 369
    //SINK
    let offset = base_value.rem_euclid(validated_divisor);
    format!("{}", offset)
}

#[derive(Deserialize)]
pub struct FilePayload {
    pub content: String,
}

#[post("/savedatafile?<filepath>", data = "<payload>")]
//CWE 732
//SOURCE
pub fn save_data_file(filepath: String, payload: Json<FilePayload>) -> Result<String, Status> {
    use std::os::unix::fs::PermissionsExt;
    
    let validated_path_length = validate_string_length(filepath);
    let validated_filepath = validate_safe_characters(validated_path_length);
    
    let mut file = File::create(&validated_filepath)
        .map_err(|_| Status::InternalServerError)?;
    
    file.write_all(payload.content.as_bytes())
        .map_err(|_| Status::InternalServerError)?;
    
    let permissions = std::fs::Permissions::from_mode(0o644);

    //CWE 732
    //SINK
    std::fs::set_permissions(&validated_filepath, permissions)
        .map_err(|_| Status::InternalServerError)?;
    
    Ok("Data file created".to_string())
}

#[post("/processoffset?<iterations>")]
//CWE 606
//SOURCE
pub fn process_offset(iterations: i32) -> Result<String, Status> {
    let validated_positive = validate_positive_integer(iterations);
    let validated_iterations = validate_integer_range(validated_positive);
    
    let mut counter = 0;
    //CWE 606
    //SINK
    while counter < validated_iterations {
        env::set_var("CURRENT_OFFSET", counter.to_string());
        counter += 1;
    }
    Ok("Offset processing completed".to_string())
}

#[get("/checkmemavailability?<size>")]
//CWE 789
//SOURCE
pub fn check_memory_availability(size: usize) -> Status {
    let validated_size = validate_memory_size(size);
    
    let mut buffer: Vec<u8> = Vec::new();
    //CWE 789
    //SINK
    buffer.reserve(validated_size);
    Status::Ok
}

#[get("/runcustomcode?<expression>")]
//CWE 94
//SOURCE
pub fn run_custom_code(expression: String) -> String {
    let validated_length = validate_string_length(expression);
    let validated_expression = validate_safe_characters(validated_length);
    
    let engine = Engine::new();
    //CWE 94
    //SINK
    match engine.eval_expression::<i64>(&validated_expression) {
        Ok(result) => result.to_string(),
        Err(e) => e.to_string(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct UserClaims {
    user_id: String,
    role: String,
}

#[post("/refreshtoken", data = "<token>")]
//CWE 347
//SOURCE
pub fn refresh_token(token: String) -> Result<String, Status> {
    //CWE 347
    //SINK
    let metadata = Token::decode_metadata(&token)
        .map_err(|_| Status::BadRequest)?;
    
    let claims = UserClaims {
        user_id: format!("{}", metadata.key_id().unwrap_or("unknown")),
        role: "user".to_string(),
    };
    //CWE 330
    //SOURCE
    let mut rng = StdRng::seed_from_u64(12345);
    //CWE 330
    //SINK
    let private_key = RsaPrivateKey::new(&mut rng, 2048)
        .map_err(|_| Status::InternalServerError)?;
    
    let key_pair = RS256KeyPair::from_der(
        &private_key.to_pkcs1_der()
            .map_err(|_| Status::InternalServerError)?
            .as_bytes()
    ).map_err(|_| Status::InternalServerError)?;
    
    let new_claims = Claims::with_custom_claims(claims, Duration::from_days(365));
    let refreshed_token = key_pair.sign(new_claims)
        .map_err(|_| Status::InternalServerError)?;
    
    Ok(refreshed_token)
}
