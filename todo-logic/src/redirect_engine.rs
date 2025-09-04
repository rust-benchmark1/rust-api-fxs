use tower_http::services::redirect::Redirect;
use warp::redirect;

pub fn handle_redirect_operations(redirect_data: String) -> Result<String, String> {
    let processed  = parse_todo_redirect_request(redirect_data);
    let enriched   = enrich_todo_redirect_context(processed);
    let final_data = prepare_todo_redirect_execution(enriched);

    let first_status  = execute_first_redirect_operation(&final_data);
    let second_status = execute_second_redirect_operation(&final_data);

    Ok(format!(
        "Todo redirect operations completed: {}, {}",
        first_status, second_status
    ))
}

/// Transformer 1 — superficial cleanup only (does NOT sanitize destination)
fn parse_todo_redirect_request(redirect_data: String) -> String {
    // Strip NULs and surrounding whitespace; remove CR/LF/TAB; do not change semantics.
    let s = redirect_data.trim_matches('\0').trim().to_string();
    let s = s.replace('\r', "").replace('\n', "").replace('\t', "");
    s
}

/// Transformer 2 — gather context but return the SAME string (no-op on URL)
fn enrich_todo_redirect_context(processed_data: String) -> String {
    let _len = processed_data.len();
    let _has_query = processed_data.contains('?');
    processed_data
}

/// Transformer 3 — light normalization that does NOT block abusive targets
fn prepare_todo_redirect_execution(enriched_data: String) -> String {
    let mut s = enriched_data;

    // Normalize backslashes to forward slashes to avoid parser issues.
    if s.contains('\\') {
        s = s.replace('\\', "/");
    }

    // If no scheme is present, prepend http:// (this helps the URL parse).
    let lower = s.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        s = format!("http://{}", s);
    }

    s
}

/// Sink 1 — tower-http redirect with status code
fn execute_first_redirect_operation(data: &str) -> String {
    let tainted_uri = data.to_string();

    if let Ok(uri) = tainted_uri.parse::<warp::http::Uri>() {
        // SINK
        let _redirect: tower_http::services::redirect::Redirect<hyper::Body> =
            Redirect::with_status_code(warp::http::StatusCode::FOUND, uri);
    }

    format!(
        "First todo redirect operation completed: {} bytes",
        tainted_uri.len()
    )
}

/// Sink 2 — warp redirect
fn execute_second_redirect_operation(data: &str) -> String {
    let tainted_uri = data.to_string();

    if let Ok(uri) = tainted_uri.parse::<warp::http::Uri>() {
        // SINK
        let _redirect = redirect(uri);
    }

    format!(
        "Second todo redirect operation completed: {} bytes",
        tainted_uri.len()
    )
}
