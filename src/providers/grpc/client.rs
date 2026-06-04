use crate::infra::errors::AppError;
use hello_pb::hello::greeter_client::GreeterClient;
use tokio::sync::OnceCell;
use std::time::Duration;
use tonic::transport::{Channel, Endpoint};

#[derive(Clone)]
pub struct TargetServices {
    pub greeter_addr: String,
}

pub struct GrpcClientManager {
    target: TargetServices,
    greeter_client: OnceCell<GreeterClient<Channel>>,
}

async fn init_channel(addr: &str) -> Result<Channel, AppError> {
    let channel = Endpoint::from_shared(addr.to_string())
        .map_err(|e| AppError::Internal(format!("invalid service uri err: {}", e)))?
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(5))
        .keep_alive_while_idle(true)
        .connect()
        .await
        .map_err(AppError::GrpcTransport)?;
    Ok(channel)
}

async fn init_greeter_client(greeter_addr: &str) -> Result<GreeterClient<Channel>, AppError> {
    let channel = init_channel(greeter_addr).await?;
    Ok(GreeterClient::new(channel))
}

impl GrpcClientManager {
    pub fn new(target: TargetServices) -> Self {
        Self {
            target,
            greeter_client: OnceCell::const_new(),
        }
    }

    pub async fn greeter_client(&self) -> Result<GreeterClient<Channel>, AppError> {
        self.greeter_client
            .get_or_try_init(|| async {
                Ok(init_greeter_client(&self.target.greeter_addr).await?)
            })
            .await
            .map(|c| c.clone())
    }
}
