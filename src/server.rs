use anyhow::Result;
use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum::http::Method;
use serde::Deserialize;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

use crate::{
    frontend::{
        Collection, Collector, CreatorStats, FrontendService, FrontendTopBuyer, VolumePoint,
    },
    load_client_from_env,
};

pub type AppSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

#[derive(Clone)]
pub struct AppState {
    schema: AppSchema,
    frontend: FrontendService,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn creator_stats(
        &self,
        ctx: &Context<'_>,
        wallet: String,
    ) -> async_graphql::Result<CreatorStats> {
        let service = ctx.data_unchecked::<FrontendService>();
        service.creator_stats(wallet).await.map_err(Into::into)
    }

    async fn volume_data(
        &self,
        ctx: &Context<'_>,
        wallet: String,
    ) -> async_graphql::Result<Vec<VolumePoint>> {
        let service = ctx.data_unchecked::<FrontendService>();
        service.volume_data(wallet).await.map_err(Into::into)
    }

    async fn top_buyers(
        &self,
        ctx: &Context<'_>,
        wallet: String,
        #[graphql(default = 10)] top_n: u32,
    ) -> async_graphql::Result<Vec<FrontendTopBuyer>> {
        let service = ctx.data_unchecked::<FrontendService>();
        service.top_buyers(wallet, top_n).await.map_err(Into::into)
    }

    async fn collectors(
        &self,
        ctx: &Context<'_>,
        wallet: String,
        #[graphql(default = 10)] count: u32,
    ) -> async_graphql::Result<Vec<Collector>> {
        let service = ctx.data_unchecked::<FrontendService>();
        service.collectors(wallet, count).await.map_err(Into::into)
    }

    async fn collections(
        &self,
        ctx: &Context<'_>,
        wallet: String,
        #[graphql(default = 20)] count: u32,
    ) -> async_graphql::Result<Vec<Collection>> {
        let service = ctx.data_unchecked::<FrontendService>();
        service.collections(wallet, count).await.map_err(Into::into)
    }
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    top_n: Option<u32>,
    count: Option<u32>,
}

pub async fn run_from_env() -> Result<()> {
    dotenvy::dotenv().ok();
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3001);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let client = load_client_from_env()?;
    let frontend = FrontendService::new(client);
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(frontend.clone())
        .finish();
    let state = AppState { schema, frontend };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        // GraphQL
        .route("/graphql", post(graphql_handler))
        .route("/graphql/playground", get(graphql_playground))
        // REST
        .route("/api/creator/{wallet}",             get(rest_creator_stats))
        .route("/api/creator/{wallet}/volume",      get(rest_volume_data))
        .route("/api/creator/{wallet}/top-buyers",  get(rest_top_buyers))
        .route("/api/creator/{wallet}/collectors",  get(rest_collectors))
        .route("/api/creator/{wallet}/collections", get(rest_collections))
        // Health
        .route("/health", get(|| async { "ok" }))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("zora-aomi-tools server listening on http://{addr}");
    println!("GraphQL endpoint:    http://{addr}/graphql");
    println!("GraphQL playground:  http://{addr}/graphql/playground");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn graphql_handler(
    State(state): State<AppState>,
    request: GraphQLRequest,
) -> GraphQLResponse {
    state.schema.execute(request.into_inner()).await.into()
}

async fn graphql_playground() -> impl IntoResponse {
    axum::response::Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}

async fn rest_creator_stats(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
) -> Result<Json<CreatorStats>, ApiError> {
    Ok(Json(state.frontend.creator_stats(wallet).await?))
}

async fn rest_volume_data(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
) -> Result<Json<Vec<VolumePoint>>, ApiError> {
    Ok(Json(state.frontend.volume_data(wallet).await?))
}

async fn rest_top_buyers(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<FrontendTopBuyer>>, ApiError> {
    Ok(Json(
        state
            .frontend
            .top_buyers(wallet, query.top_n.unwrap_or(10))
            .await?,
    ))
}

async fn rest_collectors(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<Collector>>, ApiError> {
    Ok(Json(
        state
            .frontend
            .collectors(wallet, query.count.unwrap_or(10))
            .await?,
    ))
}

async fn rest_collections(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<Collection>>, ApiError> {
    Ok(Json(
        state
            .frontend
            .collections(wallet, query.count.unwrap_or(20))
            .await?,
    ))
}

pub struct ApiError(anyhow::Error);

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            axum::http::StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": self.0.to_string() })),
        )
            .into_response()
    }
}