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
            vulnerable_HTTP_server::main().await;
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

    // 2. Force a panic with division by zero
    let crash_response = client
        .post(format!("{}/math", base_url))
        .json(&json!({
            "a": 1,
            "b": 0,
            "operation": "division"
        }))
        .send()
        .await;
    
    // Server should crash here
    assert!(crash_response.is_err());
    println!("Server crashed as expected");

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