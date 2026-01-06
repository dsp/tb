//! Frontend asset serving.

use axum::body::Body;
use axum::extract::Path;
use axum::http::{header, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use rust_embed::{Embed, EmbeddedFile};

#[derive(Embed)]
#[folder = "frontend/"]
#[exclude = "src/*"]
#[exclude = "*.ts"]
struct FrontendAssets;

fn get_asset(path: &str) -> Option<EmbeddedFile> {
    <FrontendAssets as Embed>::get(path)
}

/// Serve frontend assets or fallback to index.html for SPA routing.
pub async fn serve_frontend(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    serve_asset(path)
}

/// Serve account detail page with HTMX auto-loading.
pub async fn serve_account_page(Path(id): Path<String>) -> impl IntoResponse {
    Html(generate_detail_page(&format!("/api/v1/accounts/{}", id), "account"))
}

/// Serve transfer detail page with HTMX auto-loading.
pub async fn serve_transfer_page(Path(id): Path<String>) -> impl IntoResponse {
    Html(generate_detail_page(&format!("/api/v1/transfers/{}", id), "transfer"))
}

/// Generate an HTML page that loads detail content via HTMX.
fn generate_detail_page(api_url: &str, page_type: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>TigerBeetle Web - {page_type}</title>
    <link rel="stylesheet" href="/style.css">
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.1/dist/chart.umd.min.js"></script>
</head>
<body>
    <div class="container">
        <header>
            <h1>TigerBeetle Web</h1>
            <nav>
                <a href="/" class="nav-link">Dashboard</a>
                <a href="/accounts" class="nav-link">Accounts</a>
                <a href="/transfers" class="nav-link">Transfers</a>
            </nav>
        </header>

        <main id="main">
            <div hx-get="{api_url}" hx-trigger="load" hx-swap="innerHTML">
                <div class="loading">Loading {page_type} details...</div>
            </div>
        </main>

        <footer>
            <p>TigerBeetle Web Interface</p>
        </footer>
    </div>

    <script src="/dist/main.js" type="module"></script>
</body>
</html>"##,
        page_type = page_type,
        api_url = api_url
    )
}

fn serve_asset(path: &str) -> Response {
    match get_asset(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => {
            // SPA fallback - serve index.html for client-side routing
            if let Some(content) = get_asset("index.html") {
                Response::builder()
                    .header(header::CONTENT_TYPE, "text/html")
                    .body(Body::from(content.data.into_owned()))
                    .unwrap()
            } else {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Not Found"))
                    .unwrap()
            }
        }
    }
}
