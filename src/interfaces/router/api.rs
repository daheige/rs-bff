use crate::infra::config::AppState;
use crate::interfaces::handler::Reply;
use crate::interfaces::handler::{EmptyObject, greeter};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, Router, routing::get};

pub fn set_greeter_router(state: AppState) -> Router {
    // http://127.0.0.1:8080/api/v1/greeter/say/heige
    let router = Router::new()
        .route("/say/{name}", get(greeter::get_user))
        .with_state(state);
    router
}

pub fn set_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/", get(home))
        .nest("/greeter", set_greeter_router(state))
        .fallback(api_not_found); // set api group and not found handler for api/xxx

    let router = Router::new()
        .nest("/api/v1", api_routes)
        .fallback(not_found_handler); // global router not found

    router
}

// handler not found for global router not found
async fn not_found_handler() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "this page not found")
}

// handler not found
async fn api_not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(Reply {
            code: 404,
            message: "api not found".to_string(),
            data: Some(EmptyObject {}),
        }),
    )
}

async fn home() -> impl IntoResponse {
    (StatusCode::OK, "hello,bff")
}
