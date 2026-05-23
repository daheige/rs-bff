use crate::infra::errors::AppError;
use hello_pb::hello::greeter_client::GreeterClient;
use tonic::transport::Channel;

// grpc client客户端管理
#[derive(Clone)]
pub struct GrpcClientManager {
    greeter_client: GreeterClient<Channel>,
}

pub struct TargetServices {
    pub greeter_addr: String,
}

impl GrpcClientManager {
    // GreeterClient::new (高级定制)
    // 这种方式更加灵活，允许你完全控制底层的 Channel 或 GrpcService。
    // 你需要先手动创建并配置 Channel，然后将其传递给 new。
    //
    // ‌功能‌：接收一个已经创建好的 Channel（或任何实现了 GrpcService 的类型），并将其包装为具体的 gRPC 客户端。
    // ‌特点‌：
    // ‌高度可配置‌：你可以在创建 Channel 时设置 TLS、超时、并发限制、用户代理、
    //           拦截器（Interceptors）、负载均衡策略等。
    // ‌分离关注点‌：连接逻辑（Channel 创建）与服务调用逻辑（Client 创建）分离。
    // ‌同步构造‌：GreeterClient::new 本身通常是同步的（取决于具体版本实现，但主要是包装操作），
    // 因为连接工作已经在 Channel::connect() 中完成了。
    // ‌适用场景‌：生产环境、需要 mTLS 认证、需要添加请求拦截器（如 Auth Header）、
    // 需要自定义重试/限流策略、或使用 Tower 中间件栈的场景。
    pub async fn new(p : TargetServices) -> Result<Self, AppError> {
        // 客户端连接
        let greeter_client = Channel::from_shared(p.greeter_addr)
            .map_err(|e| AppError::Internal(format!("invalid hello service uri: {}", e)))?
            .timeout(std::time::Duration::from_secs(5))
            .connect()
            .await?;

        Ok(Self {
            greeter_client: GreeterClient::new(greeter_client),
        })
    }

    pub fn greeter_client(&self) -> GreeterClient<Channel> {
        self.greeter_client.clone()
    }
}
