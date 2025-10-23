use actix_web::{
    delete, get,
    http::StatusCode,
    middleware::Logger,
    patch, post, web,
    web::{Data, Json, Path, Query},
    App, Either, HttpResponse, HttpServer, Responder, ResponseError,
};
use log::debug;
use simplelog::{Config, LevelFilter, SimpleLogger};
use std::{fmt::Display, sync::Arc};
use todo_logic::{IdentifyableTodoItem, Pagination, TodoItem, TodoStore, TodoStoreError, UpdateTodoItem};
use tokio::sync::RwLock;

use actix_web::body::BoxBody;
use serde::Deserialize;

use actix_cors::Cors;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;

/// Type for our shared state
type Db = Arc<RwLock<TodoStore>>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging.
    // Actix's Logger middleware (https://actix.rs/actix-web/actix_web/middleware/struct.Logger.html)
    // uses the log crate (https://crates.io/crates/log) to log requests. You can use any
    // compatible logger, but for this example we'll use simplelog.
    SimpleLogger::init(LevelFilter::Debug, Config::default()).unwrap();

    // Create shared data store
    let state = Data::new(Db::default());

    let secret_key = Key::generate();

    HttpServer::new(move || {
        App::new()
            // Register a middleware to log requests.
            // More about writing custom middleware at https://actix.rs/docs/middleware/
            .wrap(Logger::default())

            // CWE 942
            //SINK
            .wrap(Cors::permissive())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    // CWE 614
                    //SINK
                    .cookie_secure(false)
                    // CWE 1004
                    //SINK
                    .cookie_http_only(false)
                    .build()
            )

            // Register our shared state.
            // More about using shared state at https://actix.rs/docs/application/
            .app_data(state.clone())
            // Register our routes. Actix supports working with (service)
            // and without macros (route).
            .service(get_todos)
            .service(add_todo)
            .service(delete_todo)
            .service(update_todo)
            .service(persist)
            .service(search_todos)
            .service(list_configs)
            .route("/todos/item/{id}", web::get().to(get_todo))
    })
    // Start the server.
    // More about server at https://actix.rs/docs/server/
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}

/// Get list of todo items
///
/// Note the use of Extractors to extract data from the query
/// and the shared state. More about extractors at
/// https://actix.rs/docs/extractors/.
///
/// Also note the Responder trait (https://actix.rs/docs/extractors/).
/// Actix comes with a lot of built-in responders, but you can also
/// implement your own.
#[get("/todos")]
async fn get_todos(pagination: Query<Pagination>, db: Data<Db>) -> impl Responder {
    let todos = db.read().await;
    let Query(pagination) = pagination;
    Json(todos.get_todos(pagination))
}

/// If a method returns different return types, Actix offers
/// the Either enum (https://actix.rs/docs/handlers/).
type ItemOrStatus = Either<Json<IdentifyableTodoItem>, HttpResponse>;

/// Get a single todo item
async fn get_todo(id: Path<usize>, db: Data<Db>) -> ItemOrStatus {
    let todos = db.read().await;
    if let Some(item) = todos.get_todo(*id) {
        Either::Left(Json(item.clone()))
    } else {
        // Use HttpResponse to build responses with status code,
        // body, headers, etc.
        Either::Right(HttpResponse::NotFound().body("Not found"))
    }
}

/// Add a new todo item
///
/// Note the use of the Json extractor to extract the body.
#[post("/todos")]
async fn add_todo(db: Data<Db>, todo: Json<TodoItem>) -> impl Responder {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo.clone());
    HttpResponse::Created().json(todo)
}

/// Delete a todo item
///
/// Note the use of another Extractor, Path, to extract the id.
#[delete("/todos/item/{id}")]
async fn delete_todo(id: Path<usize>, db: Data<Db>) -> impl Responder {
    match db.write().await.remove_todo(*id) {
        Some(_) => HttpResponse::NoContent(),
        None => HttpResponse::NotFound(),
    }
}

/// Update a todo item
#[patch("/todos/item/{id}")]
async fn update_todo(id: Path<usize>, db: Data<Db>, input: Json<UpdateTodoItem>) -> ItemOrStatus {
    let mut todos = db.write().await;
    let res = todos.update_todo(&id, input.into_inner());
    match res {
        Some(todo) => Either::Left(Json(todo.clone())),
        None => Either::Right(HttpResponse::NotFound().finish()),
    }
}

/// Application-level error object
#[derive(Debug)]
enum AppError {
    TodoStore(TodoStoreError),
    // In practice, we would have more error types here.
}
impl From<TodoStoreError> for AppError {
    fn from(inner: TodoStoreError) -> Self {
        AppError::TodoStore(inner)
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::TodoStore(e) => write!(f, "Todo store related error: {e}"),
            // In practice, we would have more error types here.
        }
    }
}

/// Implement a custom error response.
///
/// More about error handling at https://actix.rs/docs/errors/.
impl ResponseError for AppError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code()).json(match self {
            AppError::TodoStore(e) => match e {
                TodoStoreError::FileAccessError(_) => "Error while writing to file",
                TodoStoreError::SerializationError(_) => "Error during serialization",
            },
        })
    }
}

/// Persist the todo store to disk
///
/// Note the return type here. We can return our custom error type
/// AppError as it implements ResponseError.
#[post("/todos/persist")]
async fn persist(db: Data<Db>) -> Result<&'static str, AppError> {
    // Write a log message
    debug!("Persisting todos");

    let todos = db.read().await;
    todos.persist().await?;
    Ok("")
}

#[derive(serde::Deserialize)]
struct FilterQuery {
    filter: Option<String>,
}

#[get("/todos/search")]
async fn search_todos(query: Query<FilterQuery>) -> impl Responder {
    // Hard-coded todo list (as requested)
    let todos = vec![
        "Fix bug #142",
        "Write unit tests",
        "Call",
        "Deploy to staging",
    ];
    // CWE 79
    //SOURCE
    let filter_opt = query.filter.as_ref();

    // If filter is present and non-empty, filter by substring; otherwise show all.
    let filtered: Vec<&str> = match filter_opt {
        Some(f) if !f.is_empty() => todos.into_iter().filter(|item| item.contains(f)).collect(),
        _ => todos,
    };

    // Intentionally use raw filter value in the HTML
    let search_display = filter_opt.map(|s| s.as_str()).unwrap_or("");

    // Build HTML with inline CSS
    let html = format!(r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Todos</title>
  <style>
    :root {{ --bg:#0f172a; --card:#0b1220; --muted:#94a3b8; --accent:#60a5fa; }}
    body {{ margin:0; font-family:Inter,ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,"Helvetica Neue",Arial; background:linear-gradient(180deg, #071129 0%, var(--bg) 100%); color:#e6eef8; }}
    .container {{ max-width:900px; margin:40px auto; padding:24px; }}
    .card {{ background:var(--card); border-radius:12px; padding:20px; box-shadow:0 6px 18px rgba(2,6,23,0.6); }}
    h1 {{ margin:0 0 8px 0; font-size:22px; }}
    .search {{ margin:14px 0; }}
    .search .label {{ color:var(--muted); font-size:13px; }}
    .search .value {{ margin-top:6px; padding:10px 12px; background:#071025; border-radius:8px; font-family:monospace; }}
    ul.todos {{ list-style:none; padding:0; margin:12px 0 0 0; display:grid; gap:8px; }}
    ul.todos li {{ padding:12px 14px; background:linear-gradient(180deg, rgba(255,255,255,0.02), rgba(255,255,255,0.01)); border-radius:8px; }}
    .muted {{ color:var(--muted); font-size:13px; }}
    footer {{ margin-top:12px; color:var(--muted); font-size:12px; }}
  </style>
</head>
<body>
  <div class="container">
    <div class="card">
      <h1>Todo List</h1>
      <div class="search">
        <div class="label">Search parameter:</div>
        <!-- VULNERABLE: inserting user-supplied filter directly into the page -->
        <div class="value">{search_display}</div>
      </div>

      <div class="muted">Showing {count} items{filtered_note}.</div>
      <ul class="todos">
        {items_html}
      </ul>
    </div>
  </div>
</body>
</html>"#,
        search_display = search_display,
        count = filtered.len(),
        filtered_note = if filter_opt.is_some() { " (filtered)" } else { "" },
        items_html = filtered.iter().map(|s| format!("<li>{}</li>", s)).collect::<String>()
    );

    // CWE 79
    //SINK
    match HttpResponse::Ok().content_type("text/html; charset=utf-8").message_body(BoxBody::new(html))
    {
        Ok(resp) => resp,
        Err(_) => HttpResponse::InternalServerError().body("Failed to build response"),
    }
}

#[derive(Deserialize)]
pub struct ConfigQuery {
    pub filter: Option<String>,
}

#[get("/configs")]
async fn list_configs(query: Query<ConfigQuery>) -> impl Responder {
    let configs = vec![
        "database_url=postgres://localhost:5432",
        "cache_enabled=true",
        "api_key=12345-ABCDE",
        "feature_x_enabled=false",
        "log_level=debug",
        "max_connections=100",
    ];

    // CWE 79
    //SOURCE
    let filter_text = query.filter.clone().unwrap_or_else(|| "".to_string());


    let filtered: Vec<&str> = if filter_text.is_empty() {
        configs.iter().map(|s| *s).collect()
    } else {
        configs
            .iter()
            .filter(|c| c.contains(&filter_text))
            .map(|s| *s)
            .collect()
    };


    let html = format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Available Configurations</title>
            <style>
                body {{
                    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
                    background: linear-gradient(135deg, #74ABE2, #5563DE);
                    color: #fff;
                    display: flex;
                    flex-direction: column;
                    align-items: center;
                    min-height: 100vh;
                    margin: 0;
                    padding: 2rem;
                }}
                h1 {{
                    margin-bottom: 1rem;
                    text-shadow: 1px 1px 2px rgba(0,0,0,0.3);
                }}
                form {{
                    margin-bottom: 2rem;
                }}
                input[type="text"] {{
                    padding: 0.6rem 1rem;
                    border: none;
                    border-radius: 20px;
                    width: 240px;
                    outline: none;
                }}
                button {{
                    padding: 0.6rem 1.2rem;
                    border: none;
                    border-radius: 20px;
                    background-color: #fff;
                    color: #5563DE;
                    cursor: pointer;
                    font-weight: bold;
                    margin-left: 0.5rem;
                    transition: background 0.3s;
                }}
                button:hover {{
                    background-color: #e0e0e0;
                }}
                ul {{
                    list-style-type: none;
                    background: rgba(255,255,255,0.1);
                    border-radius: 10px;
                    padding: 1rem 2rem;
                    box-shadow: 0 4px 15px rgba(0,0,0,0.2);
                }}
                li {{
                    padding: 0.5rem 0;
                    border-bottom: 1px solid rgba(255,255,255,0.2);
                }}
                li:last-child {{
                    border-bottom: none;
                }}
                .filter-info {{
                    margin-bottom: 1rem;
                    font-style: italic;
                    opacity: 0.9;
                }}
            </style>
        </head>
        <body>
            <h1>Available Configurations</h1>
            <form method="get" action="/configs">
                <input type="text" name="filter" placeholder="Filter configs..." value="{filter_text}">
                <button type="submit">Search</button>
            </form>
            <div class="filter-info">Showing results for filter: <b>{filter_text}</b></div>
            <ul>
                {items}
            </ul>
        </body>
        </html>
        "#,
        filter_text = filter_text, // ← sem sanitização proposital (CWE-79)
        items = filtered
            .iter()
            .map(|c| format!("<li>{}</li>", c))
            .collect::<Vec<String>>()
            .join("")
    );
    // CWE 79
    //SINK
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html)
}
