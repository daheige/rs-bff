use std::sync::Arc;
use log::info;
use crate::infra::config::{AppState, Config};
use crate::infra::grpc::{GrpcClientManager, TargetServices};

pub async fn new_app_state(config: &Config) ->Result<AppState, Box<dyn std::error::Error>> {
    let user_service_addr = config.get_service_target("user")?;
    let order_service_addr = config.get_service_target("order")?;
    let greeter_service_addr = config.get_service_target("hello")?;

    info!("user service: {}", user_service_addr);
    info!("order service: {}", order_service_addr);

    let s = TargetServices{
        user_addr: user_service_addr,
        order_addr: order_service_addr,
        hello_addr: greeter_service_addr,
    };
    let grpc_manager = GrpcClientManager::new(s).await?;
    let state = AppState {
        grpc_manager: Arc::new(grpc_manager),
    };

    Ok(state)
}