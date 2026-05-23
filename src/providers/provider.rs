use std::sync::Arc;
use crate::infra::config::{AppState, Config};
use crate::providers::grpc::{GrpcClientManager, TargetServices};

pub async fn new_app_state(config: &Config) ->Result<AppState, Box<dyn std::error::Error>> {
    let greeter_addr = config.get_service_target("greeter-svc")?;
    println!("greeter address: {}", greeter_addr);

    let s = TargetServices{
        greeter_addr,
    };
    let grpc_manager = GrpcClientManager::new(s).await?;
    let state = AppState {
        grpc_manager: Arc::new(grpc_manager),
    };

    Ok(state)
}