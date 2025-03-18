use axum::{
    extract::Path, http::StatusCode, response::Json, routing::get, Router
};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Define routes
    let app = Router::new()
        .route("/last-modified/{package}", get(get_last_modified));

    // Run server on port 3000
    let addr = SocketAddr::from(([127, 0, 0, 1], 8454));
    println!("🚀 Server running at http://{}", addr);

    // Create a TcpListener
    let listener = TcpListener::bind(addr).await.unwrap();

    // Serve the application
    axum::serve(listener, app)
        .await
        .unwrap();
}

// API Handler
async fn get_last_modified(Path(package_name): Path<String>) -> Result<Json<Value>, StatusCode> {
    let owner = "void-linux";
    let repo = "void-packages";
    println!("Received package_name: {}", package_name);
    let path = format!("srcpkgs/{}/template", package_name); // File path inside the repo
    println!("Path: {}", path);
    let url = format!(
        "https://api.github.com/repos/{}/{}/commits?path={}",
        owner, repo, path
    );

    let client = Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "Rust-Reqwest") // GitHub API requires User-Agent
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .text()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let json: Value = serde_json::from_str(&response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(date_str) = json[0]["commit"]["committer"]["date"].as_str() {
        let date = DateTime::parse_from_rfc3339(date_str)
                                                     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                                                     .with_timezone(&Utc);


        let now = Utc::now();

        let duration_since_last_modified = now - date;
        let weeks_since_last_modified = duration_since_last_modified.num_days();

        Ok(Json(json!({ 
            "package_name": path,
            "last_modified": date_str,
            "days_since_last_modifed": weeks_since_last_modified
         })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}