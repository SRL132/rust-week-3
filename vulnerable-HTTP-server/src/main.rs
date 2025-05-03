use axum::{
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::sync::Mutex as AsyncMutex;
use std::sync::Arc;
use std::collections::HashMap;
use std::process;
use std::panic;

#[derive(Deserialize)]
struct MathQuery {
    a: u64,
    b: u64,
    operation: String,
}

#[derive(Serialize)]
struct MathResult {
    result: u64,
}

type Storage = Arc<AsyncMutex<HashMap<String, serde_json::Value>>>;

async fn calculate(Json(payload): Json<MathQuery>) -> Json<MathResult> {
    let result = match payload.operation.as_str() {
        "addition" => payload.a + payload.b,
        "subtraction" => payload.a - payload.b,
        "multiplication" => payload.a * payload.b, //can force overflow when multiplying two large numbers
        "division" => payload.a / payload.b, //can force division by zero when dividing by zero
        _ => u64::MAX,
    };

    Json(MathResult { result })
}

async fn store_data(
    Json(data): Json<serde_json::Value>,
    storage: Arc<AsyncMutex<HashMap<String, serde_json::Value>>>,
) -> String {
    let mut store = storage.lock().await;
    let key = format!("entry_{}", store.len() + 1);
    store.insert(key.clone(), data);
    format!("Data stored with key: {}", key)
}
//@audit-issue: this is a vulnerable endpoint that returns all data in the storage
async fn retrieve_all(storage: Arc<AsyncMutex<HashMap<String, serde_json::Value>>>) -> Json<HashMap<String, serde_json::Value>> {
    let store = storage.lock().await;
    Json(store.clone())
}

pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    //@audit this line sets a panic hook to abort the process when a thread panics
    panic::set_hook(Box::new(|_| {
        eprintln!("A thread panicked! Aborting the process...");
        process::abort();
    }));
    //@audit this line creates a new storage instance
    let storage: Storage = Arc::new(AsyncMutex::new(HashMap::new()));

    let app = Router::new()
        .route("/math", post(calculate))
        .route("/store", post({
            let storage = storage.clone();
            move |json| store_data(json, storage.clone()) 
        }))
        .route("/store/all", axum::routing::get({
            let storage = storage.clone();
            move || retrieve_all(storage.clone())
        }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_server().await {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest;
    use serde_json::json;
    use std::thread;
    use std::time::Duration;

    #[tokio::test]
    async fn test_server_crash_and_data_loss() {
        // Start server in a separate thread
        thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                run_server().await.unwrap();
            });
        });

        // Wait for server to start
        thread::sleep(Duration::from_secs(1));

        let client = reqwest::Client::new();
        let base_url = "http://localhost:3000";

        // 1. Store some data
        let store_response = client
            .post(format!("{}/store", base_url))
            .json(&json!({
                "test_data": "important information"
            }))
            .send()
            .await
            .unwrap();
        
        assert!(store_response.status().is_success());
        println!("Data stored successfully");

        // 2. Force a panic with division by zero and catch the error, can also work with a panic multiplying two large numbers
        let crash_result = client
            .post(format!("{}/math", base_url))
            .json(&json!({
                "a": 1,
                "b": 0,
                "operation": "division"
            }))
            .send()
            .await;
        
        // Verify that the request failed due to server crash
        match crash_result {
            Ok(_) => panic!("Server should have crashed!"),
            Err(e) => {
                println!("Server crashed as expected with error: {}", e);
                assert!(e.is_connect() || e.is_timeout(), "Expected connection error due to server crash");
            }
        }

        // 3. Wait a moment for server to restart
        thread::sleep(Duration::from_secs(1));

        // 4. Try to retrieve stored data
        let retrieve_response = client
            .get(format!("{}/store/all", base_url))
            .send()
            .await
            .unwrap();

        let data: serde_json::Value = retrieve_response.json().await.unwrap();
        
        // Data should be lost because server was restarted
        assert!(data.as_object().unwrap().is_empty());
        println!("Data was lost after server crash");
    }
}