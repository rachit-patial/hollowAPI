use axum::{
    extract::Path, http::StatusCode, response::Json, routing::get, Router
};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde_json::{json, Value};
use std::{fs::File, io::{Read, Write}, net::SocketAddr};
use tokio::{fs, net::TcpListener};

const CACHE_EXPIRY_HOURS: i64 = 1;

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

    let cache_dir = "cache";
    let cache_file = format!("{}/{}.json", cache_dir, package_name);

    fs::create_dir(cache_dir)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);

    if let Ok(metadata) = tokio::fs::metadata(&cache_file).await {
        if let Ok(modified) = metadata.modified() {
            let modified_time: DateTime<Utc> = modified.into();
            let now = Utc::now();
            let age_hours = (now - modified_time).num_hours();

            if age_hours < CACHE_EXPIRY_HOURS {
                if let Ok(mut file) = File::open(&cache_file) {
                    let mut contents = String::new();   
                    file.read_to_string(&mut contents).unwrap(); 
                    if let Ok(cached_json) = serde_json::from_str::<Value>(&contents) {
                    println!("Using cached date for {}", package_name);
                    return Ok(Json(cached_json));
                    
                    }
                    
                }
            } else {
                println!("Cache expired for {}, fetching new data....", package_name);
            }
        }
    }

    let owner = "void-linux";
    let repo = "void-packages";
    println!("Received package_name: {}", package_name);
    let path = format!("srcpkgs/{}/template", package_name); // File path inside the repo
    //println!("Path: {}", path);
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

        let cached_data = json!({
            "package_name": path,
            "last_modified": date_str,
            "days_since_last_modifed": weeks_since_last_modified
        });

        let mut file = File::create(&cache_file).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        file.write_all(cached_data.to_string().as_bytes())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;     

        println!("Cached data for {}", package_name);

        Ok(Json(json!({ 
            "package_name": path,
            "last_modified": date_str,
            "days_since_last_modifed": weeks_since_last_modified
         })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}