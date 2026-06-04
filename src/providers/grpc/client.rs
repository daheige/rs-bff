use hello_pb::hello::greeter_client::GreeterClient;
use std::time::Duration;
use tonic::transport::{Channel, Endpoint};

pub struct TargetServices {
    pub greeter_addr: String,
}

pub struct GrpcClientManager {
    greeter_client: GreeterClient<Channel>,
}

pub fn init_channel(addr: &str) -> Channel {
    let channel = Endpoint::from_shared(addr.to_string())
        .expect(format!("failed to create endpoint: {}", addr).as_str())
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .keep_alive_while_idle(true)
        .connect_lazy();
    channel
}

impl GrpcClientManager {
    pub fn new(target: TargetServices) -> Self {
        let greeter_client = GreeterClient::new(init_channel(&target.greeter_addr));
        Self { greeter_client }
    }

    pub fn greeter_client(&self) -> GreeterClient<Channel> {
        self.greeter_client.clone()
    }
}
