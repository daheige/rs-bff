use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use crate::infra::config::AppState;
use crate::infra::errors::AppError;

#[derive(Serialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub age: i32,
    pub created_at: String,
}

impl From<crate::rust_grpc::user::User> for User {
    fn from(u: crate::rust_grpc::user::User) -> Self {
        Self {
            id: u.id,
            name: u.name,
            email: u.email,
            age: u.age,
            created_at: u.created_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateUserReq {
    pub name: String,
    pub email: String,
    pub age: i32,
}

#[derive(Serialize)]
pub struct ListUsersResp {
    pub users: Vec<User>,
    pub total: i32,
}

pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<User>, AppError> {
    let mut client = state.grpc_manager.user_client();
    let req = tonic::Request::new(crate::rust_grpc::user::GetUserRequest { id });
    let resp = client.get_user(req).await?;
    Ok(Json(resp.into_inner().into()))
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(body): Json<CreateUserReq>,
) -> Result<Json<User>, AppError> {
    let mut client = state.grpc_manager.user_client();
    let req = tonic::Request::new(crate::rust_grpc::user::CreateUserRequest {
        name: body.name,
        email: body.email,
        age: body.age,
    });
    let resp = client.create_user(req).await?;
    Ok(Json(resp.into_inner().into()))
}

pub async fn list_users(State(state): State<AppState>) -> Result<Json<ListUsersResp>, AppError> {
    let mut client = state.grpc_manager.user_client();
    let req = tonic::Request::new(crate::rust_grpc::user::ListUsersRequest {
        page: 1,
        page_size: 20,
    });
    let resp = client.list_users(req).await?;
    let inner = resp.into_inner();
    Ok(Json(ListUsersResp {
        users: inner.users.into_iter().map(Into::into).collect(),
        total: inner.total,
    }))
}