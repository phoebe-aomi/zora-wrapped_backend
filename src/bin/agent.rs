use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use zora_aomi_tools::agent::{execute_intent, parse_user_input};

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
}

#[derive(Serialize)]
struct QueryResponse {
    result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn handle_query(Json(req): Json<QueryRequest>) -> Json<QueryResponse> {
    match parse_user_input(&req.query) {
        Some(intent) => match execute_intent(&intent).await {
            Ok(result) => Json(QueryResponse { result, error: None }),
            Err(e) => Json(QueryResponse {
                result: String::new(),
                error: Some(e),
            }),
        },
        None => Json(QueryResponse {
            result: String::new(),
            error: Some("Could not understand query. Try: 'How many holders in 0x...' or 'Show top 5 buyers of 0x...'".to_string()),
        }),
    }
}

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/query", post(handle_query))
        .layer(cors);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    println!("🚀 Agent running on port {}", port);
}