use super::client::*;
use serde_json::json;

#[tokio::test]
async fn test_retry_on_500() {
    let mut server = mockito::Server::new_async().await;

    // First two attempts return 500, third returns 200
    let mock = server
        .mock("POST", "/search?key=test_key")
        .with_status(500)
        .expect(2)
        .create_async()
        .await;

    let success_mock = server
        .mock("POST", "/search?key=test_key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"test": "success"}"#)
        .expect(1)
        .create_async()
        .await;

    let mut client = InnerTube::new_with_base_url(server.url()).await.unwrap();
    client.api_key = "test_key".to_string();

    let body = json!({
        "query": "test"
    });

    let result = client.post("/search", body).await;

    mock.assert_async().await;
    success_mock.assert_async().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_success_on_200() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("POST", "/search?key=test_key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"test": "success"}"#)
        .expect(1)
        .create_async()
        .await;

    let mut client = InnerTube::new_with_base_url(server.url()).await.unwrap();
    client.api_key = "test_key".to_string();

    let body = json!({
        "query": "test"
    });

    let result = client.post("/search", body).await;

    mock.assert_async().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["test"], "success");
}

#[tokio::test]
async fn test_no_retry_on_400() {
    let mut server = mockito::Server::new_async().await;

    // Should only be called once (no retry on 4xx)
    let mock = server
        .mock("POST", "/search?key=test_key")
        .with_status(400)
        .expect(1)
        .create_async()
        .await;

    let mut client = InnerTube::new_with_base_url(server.url()).await.unwrap();
    client.api_key = "test_key".to_string();

    let body = json!({
        "query": "test"
    });

    let result = client.post("/search", body).await;

    mock.assert_async().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("400"));
}

#[tokio::test]
async fn test_no_retry_on_403() {
    let mut server = mockito::Server::new_async().await;

    // Should only be called once (no retry on 4xx)
    let mock = server
        .mock("POST", "/search?key=test_key")
        .with_status(403)
        .expect(1)
        .create_async()
        .await;

    let mut client = InnerTube::new_with_base_url(server.url()).await.unwrap();
    client.api_key = "test_key".to_string();

    let body = json!({
        "query": "test"
    });

    let result = client.post("/search", body).await;

    mock.assert_async().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("403"));
}
