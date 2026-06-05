use axum::{routing::{post, get}, extract::Path, Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use zora_aomi_tools::agent::{execute_intent, parse_user_input};
use zora_aomi_tools::server::{
    rest_creator_stats,
    rest_volume_data,
    rest_top_buyers,
    rest_collectors,
    rest_collections,
};

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

    let app: Router<()> = Router::new()
        .route("/query", post(handle_query))
        .route("/api/creator/:wallet",             get(rest_creator_stats))
        .route("/api/creator/:wallet/volume",      get(rest_volume_data))
        .route("/api/creator/:wallet/top-buyers",  get(rest_top_buyers))
        .route("/api/creator/:wallet/collectors",  get(rest_collectors))
        .route("/api/creator/:wallet/collections", get(rest_collections))
        .layer(cors);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    println!("🚀 Agent running on port {}", port);
    axum::serve(listener, app).await.unwrap();
}