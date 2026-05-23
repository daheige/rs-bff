use axum::extract::{Path, State};
use hello_pb::hello::HelloReq;
use log::info;
use crate::infra::config::AppState;
use tonic::Request;
use autometrics::autometrics;
use monitor::metrics::API_SLO;

// 提取路径中的 name 参数
// 请求方式：GET http://localhost:8080/api/v1/greeter/say/daheige
#[autometrics(objective = API_SLO)]
// 也可以使用下面的方式，简单处理
// #[autometrics]
pub async fn get_user(State(state): State<AppState>,Path(name): Path<String>) -> String {
    info!("request name: {}", name);
    // 调用rpc请求
    let mut client = state.grpc_manager.greeter_client();
    let result = client.say_hello(Request::new(HelloReq { name })).await;
    match result {
        Ok(resp) => {
            let reply = resp.into_inner();
            reply.message
        }
        Err(err) => {
            info!("request error: {}", err);
           "request error".to_string()
        }
    }
}
