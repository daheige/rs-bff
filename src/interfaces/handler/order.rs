use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use crate::infra::errors::AppError;
use crate::infra::config::AppState;

#[derive(Deserialize)]
pub struct OrderItemReq {
    pub product_id: String,
    pub product_name: String,
    pub quantity: i32,
    pub price: f64,
}

#[derive(Deserialize)]
pub struct CreateOrderReq {
    pub user_id: i64,
    pub items: Vec<OrderItemReq>,
    pub address: String,
}

#[derive(Serialize)]
pub struct OrderItem {
    pub product_id: String,
    pub product_name: String,
    pub quantity: i32,
    pub price: f64,
}

impl From<crate::rust_grpc::order::OrderItem> for OrderItem {
    fn from(item: crate::rust_grpc::order::OrderItem) -> Self {
        Self {
            product_id: item.product_id,
            product_name: item.product_name,
            quantity: item.quantity,
            price: item.price,
        }
    }
}

#[derive(Serialize)]
pub struct Order {
    pub id: i64,
    pub user_id: i64,
    pub items: Vec<OrderItem>,
    pub total_amount: f64,
    pub status: String,
    pub address: String,
    pub created_at: String,
}

impl From<crate::rust_grpc::order::Order> for Order {
    fn from(o: crate::rust_grpc::order::Order) -> Self {
        Self {
            id: o.id,
            user_id: o.user_id,
            items: o.items.into_iter().map(Into::into).collect(),
            total_amount: o.total_amount,
            status: o.status,
            address: o.address,
            created_at: o.created_at,
        }
    }
}

#[derive(Serialize)]
pub struct ListUserOrdersResp {
    pub orders: Vec<Order>,
    pub total: i32,
}

pub async fn get_order(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Order>, AppError> {
    let mut client = state.grpc_manager.order_client();
    let req = tonic::Request::new(crate::rust_grpc::order::GetOrderRequest { id });
    let resp = client.get_order(req).await?;
    Ok(Json(resp.into_inner().into()))
}

pub async fn create_order(
    State(state): State<AppState>,
    Json(body): Json<CreateOrderReq>,
) -> Result<Json<Order>, AppError> {
    let mut client = state.grpc_manager.order_client();
    let req = tonic::Request::new(crate::rust_grpc::order::CreateOrderRequest {
        user_id: body.user_id,
        items: body
            .items
            .into_iter()
            .map(|i| crate::rust_grpc::order::OrderItem {
                product_id: i.product_id,
                product_name: i.product_name,
                quantity: i.quantity,
                price: i.price,
            })
            .collect(),
        address: body.address,
    });
    let resp = client.create_order(req).await?;
    Ok(Json(resp.into_inner().into()))
}

pub async fn list_user_orders(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Result<Json<ListUserOrdersResp>, AppError> {
    let mut client = state.grpc_manager.order_client();
    let req = tonic::Request::new(crate::rust_grpc::order::ListUserOrdersRequest {
        user_id,
        page: 1,
        page_size: 20,
    });
    let resp = client.list_user_orders(req).await?;
    let inner = resp.into_inner();
    Ok(Json(ListUserOrdersResp {
        orders: inner.orders.into_iter().map(Into::into).collect(),
        total: inner.total,
    }))
}
