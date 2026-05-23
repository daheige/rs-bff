use std::sync::Arc;
use crate::providers::grpc::GrpcClientManager;

#[derive(Clone)]
pub struct AppState {
    pub grpc_manager: Arc<GrpcClientManager>,
}
