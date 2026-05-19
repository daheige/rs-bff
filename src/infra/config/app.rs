use std::sync::Arc;
use crate::infra::grpc::GrpcClientManager;

#[derive(Clone)]
pub struct AppState {
    pub grpc_manager: Arc<GrpcClientManager>,
}
