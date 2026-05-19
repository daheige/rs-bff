use axum::{routing::{get, post}, Json, Router};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use crate::interfaces::handler::{user, EmptyObject};
use crate::interfaces::handler::order;
use crate::infra::config::AppState;
use crate::interfaces::handler::Reply;

pub fn set_user_router(state: AppState) -> Router {
    let router = Router::new()
        .route("/", get(user::list_users).post(user::create_user))
        .route("/{id}", get(user::get_user))
        .with_state(state);
    router
}

pub fn set_order_router(state: AppState) -> Router{
    let router = Router::new()
        .route("/", post(order::create_order))
        .route("/{id}", get(order::get_order))
        .route("/user/{id}/list", get(order::list_user_orders))
        .with_state(state);
    router
}

pub fn set_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/",get(home))
        .nest("/user", set_user_router(state.clone()))
        .nest("/order", set_order_router(state))
        .fallback(api_not_found); // set api group and not found handler for api/xxx

    let router = Router::new()
        .nest("/api", api_routes)
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
