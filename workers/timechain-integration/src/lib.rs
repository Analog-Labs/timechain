
use reqwest::Client;
use serde_json::{json, Value};

async fn integration() -> Result<(), reqwest::Error> {
    // Create a new HTTP client
    let client = Client::new();

    // Construct the introspection query
    let query = r#"
    mutation {
      collect (
        collection:"QmWVZN1S6Yhygt35gQej6e3VbEEffbrVuqZZCQc772uRt7",
        block: 1,
        cycle: 8,
        taskId: 1,
        data: ["1"]
      ){
        status
      }
    }
    "#;

    // Send the POST request to the GraphQL server
    let response = client
        .post("http://127.0.0.1:8009/graphql")
        .json(&json!({
            "query": query
        }))
        .header("Authorization", "0;6hvzRj2aZiZUqSALPc5YgMKKrN17tRe6iGpYvRuSJzvK;0;*;;VFpTn3GQB5BMRRAZvCUrFay9kDUaFT4MpTyRvTAtPE5cASNhNVT7pMwM2XxM2eseGSVa2pWmG55TmXqNoQRi2Eb")
        .send()
        .await?;

    // Parse the response JSON
    let response_json: Value = response.json().await?;
    println!("\n\n\n{:?}\n",response_json);

    Ok(())
}

#[tokio::test]
async fn test_integration() {
    let result = integration().await;
    assert_eq!(result.is_ok(), true);
}