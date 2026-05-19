use hello_pb::hello::greeter_client::GreeterClient;
use crate::infra::errors::AppError;
use crate::rust_grpc::{
    order::order_service_client::OrderServiceClient, user::user_service_client::UserServiceClient,
};
use tonic::transport::Channel;

// grpc client客户端管理
#[derive(Clone)]
pub struct GrpcClientManager {
    user_client: UserServiceClient<Channel>,
    order_client: OrderServiceClient<Channel>,
    greeter_client: GreeterClient<Channel>,
}

pub struct TargetServices {
    pub user_addr: String,
    pub order_addr: String,
    pub hello_addr: String,
}

impl GrpcClientManager {
    pub async fn new(p : TargetServices) -> Result<Self, AppError> {
        let user_channel = Channel::from_shared(p.user_addr)
            .map_err(|e| AppError::Internal(format!("invalid user service uri: {}", e)))?
            .connect()
            .await?;

        let order_channel = Channel::from_shared(p.order_addr)
            .map_err(|e| AppError::Internal(format!("invalid order service uri: {}", e)))?
            .connect()
            .await?;

        let hello_channel = Channel::from_shared(p.hello_addr)
            .map_err(|e| AppError::Internal(format!("invalid hello service uri: {}", e)))?
            .connect()
            .await?;

        Ok(Self {
            user_client: UserServiceClient::new(user_channel),
            order_client: OrderServiceClient::new(order_channel),
            greeter_client: GreeterClient::new(hello_channel),
        })
    }

    pub fn user_client(&self) -> UserServiceClient<Channel> {
        self.user_client.clone()
    }

    pub fn order_client(&self) -> OrderServiceClient<Channel> {
        self.order_client.clone()
    }

    pub fn greeter_client(&self) -> GreeterClient<Channel> {
        self.greeter_client.clone()
    }
}
