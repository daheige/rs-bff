use crate::infra::errors::AppError;
use hello_pb::hello::greeter_client::GreeterClient;
use std::time::Duration;
use tokio::sync::OnceCell;
use tonic::transport::{Channel, Endpoint};

#[derive(Clone)]
pub struct TargetServices {
    pub greeter_addr: String,
}

pub struct GrpcClientManager {
    target: TargetServices,
    // 初始化时，不去做 gRPC 连接，只在第一次使用时建立连接
    greeter_client: OnceCell<GreeterClient<Channel>>,
}

// 创建一个基于hyper和tower服务的http2 grpc客户端
// 这个通道 Channel 类型开销较低，因为它多路服用，减少了Clone实现，它由tower_buffer::Buffer提供支持。
// 该缓冲区在后台任务中运行连接，并且提供了mpsc通道接口，相比 connect 直连方式，
// Channel 能精确设置 TLS、超时、并发限制、用户代理、拦截器（Interceptors）、负载均衡策略等。
// http2_keep_alive_interval 多久保持一次心跳
// keep_alive_timeout 心跳超时，建议：内网服务用 10 秒，公网服务用 20 秒，配合 30 秒的心跳间隔
// keep_alive_while_idle 用于保持空闲连接
// timeout 单次RPC超时
// connect_timeout 连接建立超时
async fn init_channel(addr: &str) -> Result<Channel, AppError> {
    let channel = Endpoint::from_shared(addr.to_string())
        .map_err(|e| AppError::Internal(format!("invalid service uri: {} err: {}", addr, e)))?
        .http2_keep_alive_interval(Duration::from_secs(30))
        .keep_alive_timeout(Duration::from_secs(20))
        .keep_alive_while_idle(true)
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .connect()
        .await
        .map_err(AppError::GrpcTransport)?;
    Ok(channel)
}

impl GrpcClientManager {
    pub fn new(target: TargetServices) -> Self {
        Self {
            target,
            greeter_client: OnceCell::const_new(),
        }
    }

    // 初始化greeter client
    async fn init_greeter_client(&self) -> Result<GreeterClient<Channel>, AppError> {
        let channel = init_channel(&self.target.greeter_addr).await?;
        Ok(GreeterClient::new(channel))
    }

    pub async fn greeter_client(&self) -> Result<GreeterClient<Channel>, AppError> {
        self.greeter_client
            .get_or_try_init(|| async {
                // 如果没有建立 gRPC client，就执行一次
                self.init_greeter_client().await
            })
            .await
            .map(|c| c.clone())
    }
}
