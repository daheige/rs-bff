use crate::infra::errors::AppError;
use hello_pb::hello::greeter_client::GreeterClient;
use tokio::sync::OnceCell;
use tonic::transport::Channel;

#[derive(Clone)]
pub struct TargetServices {
    pub greeter_addr: String,
}

pub struct GrpcClientManager {
    target: TargetServices,
    greeter_client: OnceCell<GreeterClient<Channel>>,
}

pub async fn init_greeter_client(greeter_addr: &str) -> Result<GreeterClient<Channel>, AppError> {
    let channel = Channel::from_shared(greeter_addr.to_string())
        .map_err(|e| AppError::Internal(format!("invalid hello service uri: {}", e)))?
        .timeout(std::time::Duration::from_secs(5))
        .connect()
        .await
        .map_err(AppError::GrpcTransport)?;

    Ok(GreeterClient::new(channel))
}

impl GrpcClientManager {
    pub fn new(target: TargetServices) -> Self {
        Self {
            target,
            greeter_client: OnceCell::const_new(),
        }
    }

    // 获取grpc client客户端（首次使用时连接，失败可重试）
    pub async fn greeter_client(&self) -> Result<GreeterClient<Channel>, AppError> {
        self.greeter_client
            .get_or_try_init(|| async {
                // println!("first init client");
                init_greeter_client(&self.target.greeter_addr).await
            })
            .await
            .map(|c| c.clone())
    }
}
