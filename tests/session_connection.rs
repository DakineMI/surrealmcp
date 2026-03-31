use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::json;

const MCP_SERVER_URL: &str = "http://127.0.0.1:8090/mcp";
const MCP_HEALTH_URL: &str = "http://127.0.0.1:8090/health";
const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

async fn is_server_available() -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok();
    
    match client {
        Some(client) => client.get(MCP_HEALTH_URL).send().await.is_ok(),
        None => false,
    }
}

fn create_mcp_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    headers.insert(
        reqwest::header::ACCEPT,
        HeaderValue::from_static("application/json, text/event-stream"),
    );
    headers.insert(
        HeaderName::from_static("mcp-protocol-version"),
        HeaderValue::from_static(MCP_PROTOCOL_VERSION),
    );
    headers
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: usize,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[serde(default)]
    jsonrpc: Option<String>,
    #[serde(default)]
    id: Option<serde_json::Value>,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct InitializeResult {
    protocol_version: String,
    capabilities: serde_json::Value,
    server_info: ServerInfo,
}

#[derive(Debug, Deserialize)]
struct ServerInfo {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct ToolsListResult {
    tools: Vec<ToolDefinition>,
}

#[derive(Debug, Deserialize)]
struct ToolDefinition {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    input_schema: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ToolCallResult {
    content: Vec<ToolContent>,
}

#[derive(Debug, Deserialize)]
struct ToolContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
}

async fn send_jsonrpc_request(
    client: &reqwest::Client,
    method: &str,
    id: usize,
    params: Option<serde_json::Value>,
) -> Result<JsonRpcResponse, reqwest::Error> {
    let request = JsonRpcRequest {
        jsonrpc: "2.0",
        id,
        method: method.to_string(),
        params,
    };

    client
        .post(MCP_SERVER_URL)
        .headers(create_mcp_headers())
        .json(&request)
        .send()
        .await?
        .json()
        .await
}

async fn send_notification(client: &reqwest::Client, method: &str) -> Result<(), reqwest::Error> {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
    });

    let method_owned = method.to_string();
    let _response = client
        .post(MCP_SERVER_URL)
        .headers(create_mcp_headers())
        .json(&request)
        .send()
        .await?;

    let _ = method_owned;
    Ok(())
}

async fn initialize_session(client: &reqwest::Client) -> Result<InitializeResult, Box<dyn std::error::Error>> {
    let response = send_jsonrpc_request(
        client,
        "initialize",
        1,
        Some(json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        })),
    )
    .await?;

    assert!(response.error.is_none(), "Initialize failed: {:?}", response.error);
    let result = response.result.unwrap();
    Ok(serde_json::from_value(result)?)
}

#[tokio::test]
async fn test_mcp_initialize() {
    if !is_server_available().await {
        eprintln!("Server not available at {}. Run 'cargo run' first.", MCP_HEALTH_URL);
        return;
    }
    
    let client = reqwest::Client::new();

    let response = send_jsonrpc_request(
        &client,
        "initialize",
        1,
        Some(json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        })),
    )
    .await
    .expect("Failed to send initialize request");

    assert!(
        response.error.is_none(),
        "Initialize returned error: {:?}",
        response.error
    );
    assert!(
        response.result.is_some(),
        "Initialize should return a result"
    );

    let result = response.result.unwrap();
    let init_result: InitializeResult =
        serde_json::from_value(result).expect("Failed to parse initialize result");

    assert_eq!(init_result.protocol_version, MCP_PROTOCOL_VERSION);
    assert!(!init_result.server_info.name.is_empty());
    assert!(!init_result.server_info.version.is_empty());
    assert!(
        init_result.capabilities.get("roots").is_some()
            || init_result.capabilities.get("tools").is_some()
            || init_result.capabilities.get("prompts").is_some()
    );

    send_notification(&client, "notifications/initialized")
        .await
        .expect("Failed to send initialized notification");
}

#[tokio::test]
async fn test_mcp_tools_list() {
    if !is_server_available().await {
        eprintln!("Server not available at {}. Run 'cargo run' first.", MCP_HEALTH_URL);
        return;
    }
    
    let client = reqwest::Client::new();

    initialize_session(&client).await.expect("Failed to initialize session");

    let response = send_jsonrpc_request(&client, "tools/list", 2, Some(json!({})))
        .await
        .expect("Failed to send tools/list request");

    assert!(response.error.is_none(), "tools/list returned error: {:?}", response.error);
    assert!(response.result.is_some(), "tools/list should return a result");

    let result = response.result.unwrap();
    let tools_result: ToolsListResult =
        serde_json::from_value(result).expect("Failed to parse tools list result");

    assert!(!tools_result.tools.is_empty(), "Server should advertise at least one tool");

    let tool_names: Vec<&str> = tools_result
        .tools
        .iter()
        .map(|t| t.name.as_str())
        .collect();

    assert!(
        tool_names.contains(&"surreal_query"),
        "Expected surreal_query tool, found: {:?}",
        tool_names
    );
    assert!(
        tool_names.contains(&"surreal_query_range"),
        "Expected surreal_query_range tool, found: {:?}",
        tool_names
    );

    for tool in &tools_result.tools {
        assert!(!tool.name.is_empty(), "Tool should have a name");
        if let Some(desc) = &tool.description {
            assert!(
                !desc.is_empty(),
                "Tool {} should have a non-empty description",
                tool.name
            );
        }
    }
}

#[tokio::test]
async fn test_mcp_tool_call_surreal_query() {
    if !is_server_available().await {
        eprintln!("Server not available at {}. Run 'cargo run' first.", MCP_HEALTH_URL);
        return;
    }
    
    let client = reqwest::Client::new();

    initialize_session(&client).await.expect("Failed to initialize session");

    let response = send_jsonrpc_request(
        &client,
        "tools/call",
        3,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": "SELECT * FROM person LIMIT 5"
            }
        })),
    )
    .await
    .expect("Failed to send tools/call request");

    assert!(
        response.error.is_none(),
        "tools/call returned error: {:?}",
        response.error
    );
    assert!(
        response.result.is_some(),
        "tools/call should return a result"
    );

    let result = response.result.unwrap();
    let call_result: ToolCallResult =
        serde_json::from_value(result).expect("Failed to parse tool call result");

    assert!(
        !call_result.content.is_empty(),
        "Tool call should return content"
    );
    assert_eq!(
        call_result.content[0].content_type, "text",
        "Content type should be text"
    );
    assert!(
        call_result.content[0].text.is_some(),
        "Content should have text"
    );

    let response_text = call_result.content[0].text.as_ref().unwrap();
    assert!(
        response_text.contains("SELECT") || response_text.contains("[]") || response_text.contains("result"),
        "Response should contain query result, got: {}",
        response_text
    );
}

#[tokio::test]
async fn test_mcp_tool_call_with_create() {
    if !is_server_available().await {
        eprintln!("Server not available at {}. Run 'cargo run' first.", MCP_HEALTH_URL);
        return;
    }
    
    let client = reqwest::Client::new();

    initialize_session(&client).await.expect("Failed to initialize session");

    let test_name = format!("test_user_{}", uuid::Uuid::new_v4());
    let create_response = send_jsonrpc_request(
        &client,
        "tools/call",
        10,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": format!("CREATE person SET name = '{}', created_at = time::now()", test_name)
            }
        })),
    )
    .await
    .expect("Failed to send CREATE request");

    assert!(
        create_response.error.is_none(),
        "CREATE returned error: {:?}",
        create_response.error
    );

    let query_response = send_jsonrpc_request(
        &client,
        "tools/call",
        11,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": format!("SELECT * FROM person WHERE name = '{}'", test_name)
            }
        })),
    )
    .await
    .expect("Failed to send SELECT request");

    assert!(
        query_response.error.is_none(),
        "SELECT returned error: {:?}",
        query_response.error
    );

    let result = query_response.result.unwrap();
    let call_result: ToolCallResult =
        serde_json::from_value(result).expect("Failed to parse tool call result");

    assert!(
        !call_result.content.is_empty(),
        "SELECT should return content"
    );

    let response_text = call_result.content[0].text.as_ref().unwrap();
    assert!(
        response_text.contains(&test_name),
        "Should find the created user '{}' in result: {}",
        test_name,
        response_text
    );
}

#[tokio::test]
async fn test_session_persistence() {
    if !is_server_available().await {
        eprintln!("Server not available at {}. Run 'cargo run' first.", MCP_HEALTH_URL);
        return;
    }
    
    let client = reqwest::Client::new();

    initialize_session(&client).await.expect("Failed to initialize session");

    let test_id = format!("persist_{}", uuid::Uuid::new_v4());

    let create_response = send_jsonrpc_request(
        &client,
        "tools/call",
        20,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": format!("CREATE person SET test_id = '{}', value = 100", test_id)
            }
        })),
    )
    .await
    .expect("Failed to send first request");

    assert!(
        create_response.error.is_none(),
        "First request failed: {:?}",
        create_response.error
    );

    let query_response = send_jsonrpc_request(
        &client,
        "tools/call",
        21,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": format!("SELECT * FROM person WHERE test_id = '{}'", test_id)
            }
        })),
    )
    .await
    .expect("Failed to send second request");

    assert!(
        query_response.error.is_none(),
        "Second request failed: {:?}",
        query_response.error
    );

    let result = query_response.result.unwrap();
    let call_result: ToolCallResult =
        serde_json::from_value(result).expect("Failed to parse result");

    let response_text = call_result.content[0].text.as_ref().unwrap();
    assert!(
        response_text.contains(&test_id),
        "Session should persist data across requests: {}",
        response_text
    );
}

#[tokio::test]
async fn test_multiple_sessions_independent() {
    if !is_server_available().await {
        eprintln!("Server not available at {}. Run 'cargo run' first.", MCP_HEALTH_URL);
        return;
    }
    
    let client1 = reqwest::Client::new();
    let client2 = reqwest::Client::new();

    initialize_session(&client1)
        .await
        .expect("Client 1 failed to initialize");
    initialize_session(&client2)
        .await
        .expect("Client 2 failed to initialize");

    let session1_test_id = format!("session1_{}", uuid::Uuid::new_v4());
    let session2_test_id = format!("session2_{}", uuid::Uuid::new_v4());

    let create1 = send_jsonrpc_request(
        &client1,
        "tools/call",
        30,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": format!("CREATE person SET session_marker = '{}', value = 1", session1_test_id)
            }
        })),
    )
    .await
    .expect("Client 1 create failed");

    assert!(create1.error.is_none(), "Client 1 create failed: {:?}", create1.error);

    let create2 = send_jsonrpc_request(
        &client2,
        "tools/call",
        31,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": format!("CREATE person SET session_marker = '{}', value = 2", session2_test_id)
            }
        })),
    )
    .await
    .expect("Client 2 create failed");

    assert!(create2.error.is_none(), "Client 2 create failed: {:?}", create2.error);

    let query1 = send_jsonrpc_request(
        &client1,
        "tools/call",
        32,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": format!("SELECT * FROM person WHERE session_marker = '{}'", session1_test_id)
            }
        })),
    )
    .await
    .expect("Client 1 query failed");

    let query1_result: ToolCallResult =
        serde_json::from_value(query1.result.unwrap()).expect("Failed to parse client 1 result");
    let query1_text = query1_result.content[0].text.as_ref().unwrap();

    let query2 = send_jsonrpc_request(
        &client2,
        "tools/call",
        33,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": format!("SELECT * FROM person WHERE session_marker = '{}'", session2_test_id)
            }
        })),
    )
    .await
    .expect("Client 2 query failed");

    let query2_result: ToolCallResult =
        serde_json::from_value(query2.result.unwrap()).expect("Failed to parse client 2 result");
    let query2_text = query2_result.content[0].text.as_ref().unwrap();

    assert!(
        query1_text.contains(&session1_test_id),
        "Session 1 should have its data: {}",
        query1_text
    );
    assert!(
        query2_text.contains(&session2_test_id),
        "Session 2 should have its data: {}",
        query2_text
    );
}

#[tokio::test]
async fn test_mcp_invalid_tool_name() {
    if !is_server_available().await {
        eprintln!("Server not available at {}. Run 'cargo run' first.", MCP_HEALTH_URL);
        return;
    }
    
    let client = reqwest::Client::new();

    initialize_session(&client).await.expect("Failed to initialize session");

    let response = send_jsonrpc_request(
        &client,
        "tools/call",
        40,
        Some(json!({
            "name": "nonexistent_tool",
            "arguments": {}
        })),
    )
    .await
    .expect("Failed to send request");

    assert!(
        response.error.is_some(),
        "Should return error for invalid tool name"
    );
}

#[tokio::test]
async fn test_mcp_invalid_query_syntax() {
    if !is_server_available().await {
        eprintln!("Server not available at {}. Run 'cargo run' first.", MCP_HEALTH_URL);
        return;
    }
    
    let client = reqwest::Client::new();

    initialize_session(&client).await.expect("Failed to initialize session");

    let response = send_jsonrpc_request(
        &client,
        "tools/call",
        41,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": "INVALID SYNTAX HERE !!!"
            }
        })),
    )
    .await
    .expect("Failed to send request");

    assert!(
        response.error.is_some() || response.result.is_some(),
        "Should return either error or result"
    );

    if let Some(result) = response.result {
        let call_result: ToolCallResult =
            serde_json::from_value(result).expect("Failed to parse result");
        if let Some(text) = call_result.content.first().and_then(|c| c.text.clone()) {
            assert!(
                text.contains("error") || text.contains("Error") || text.contains("failed"),
                "Invalid query should return error message, got: {}",
                text
            );
        }
    }
}

#[tokio::test]
async fn test_mcp_query_with_parameters() {
    if !is_server_available().await {
        eprintln!("Server not available at {}. Run 'cargo run' first.", MCP_HEALTH_URL);
        return;
    }
    
    let client = reqwest::Client::new();

    initialize_session(&client).await.expect("Failed to initialize session");

    let response = send_jsonrpc_request(
        &client,
        "tools/call",
        50,
        Some(json!({
            "name": "surreal_query_range",
            "arguments": {
                "query": "SELECT * FROM person LIMIT 10",
                "limit": 5
            }
        })),
    )
    .await
    .expect("Failed to send request");

    assert!(
        response.error.is_none(),
        "surreal_query_range should work: {:?}",
        response.error
    );
    assert!(
        response.result.is_some(),
        "Should return a result"
    );
}

#[tokio::test]
async fn test_full_mcp_workflow() {
    if !is_server_available().await {
        eprintln!("Server not available at {}. Run 'cargo run' first.", MCP_HEALTH_URL);
        return;
    }
    
    let client = reqwest::Client::new();

    let init_response = send_jsonrpc_request(
        &client,
        "initialize",
        1,
        Some(json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "integration-test-client",
                "version": "1.0.0"
            }
        })),
    )
    .await
    .expect("Initialize failed");

    assert!(init_response.error.is_none());

    send_notification(&client, "notifications/initialized")
        .await
        .expect("Initialized notification failed");

    let tools_response = send_jsonrpc_request(&client, "tools/list", 2, Some(json!({})))
        .await
        .expect("Tools list failed");

    assert!(tools_response.error.is_none());

    let workflow_id = format!("workflow_{}", uuid::Uuid::new_v4());

    let create_response = send_jsonrpc_request(
        &client,
        "tools/call",
        3,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": format!("CREATE person SET workflow_id = '{}', status = 'started'", workflow_id)
            }
        })),
    )
    .await
    .expect("Create failed");

    assert!(create_response.error.is_none());

    let update_response = send_jsonrpc_request(
        &client,
        "tools/call",
        4,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": format!("UPDATE person SET status = 'completed' WHERE workflow_id = '{}'", workflow_id)
            }
        })),
    )
    .await
    .expect("Update failed");

    assert!(update_response.error.is_none());

    let verify_response = send_jsonrpc_request(
        &client,
        "tools/call",
        5,
        Some(json!({
            "name": "surreal_query",
            "arguments": {
                "query": format!("SELECT status FROM person WHERE workflow_id = '{}'", workflow_id)
            }
        })),
    )
    .await
    .expect("Verify failed");

    assert!(verify_response.error.is_none());

    let verify_result: ToolCallResult =
        serde_json::from_value(verify_response.result.unwrap())
            .expect("Failed to parse verify result");
    let verify_text = verify_result.content[0].text.as_ref().unwrap();

    assert!(
        verify_text.contains("completed"),
        "Workflow should be completed: {}",
        verify_text
    );
}
