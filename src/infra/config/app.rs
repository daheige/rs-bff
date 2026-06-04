use crate::providers::grpc::GrpcClientManager;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub grpc_manager: Arc<GrpcClientManager>,
}
