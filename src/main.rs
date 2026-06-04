use crate::infra::config::Config;
use crate::interfaces::router::set_router;
use chrono::Utc;
use log::info;
use monitor::metrics::prometheus_init;
use providers::provider::new_app_state;
use shutdown::graceful_shutdown;
use std::io::Write;
use std::net::SocketAddr;
use std::time::Duration;

// 定义模块
mod infra;
mod interfaces;
mod providers;
mod rust_grpc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_yaml("app.yaml")?;

    if config.app_debug {
        println!("config: {:#?}", config);
    }

    // 设置日志级别
    env_logger::Builder::new()
        .target(env_logger::Target::Stdout)
        .parse_filters(&config.log_level)
        .format(|buf, record| {
            let level = record.level();
            writeln!(
                buf,
                "[{} {} {}:{}] {}",
                Utc::now().format("%Y-%m-%dT%H:%M:%SZ"), // 时间格式
                level,                                   // 日志级别
                record.module_path().unwrap_or("unnamed"), // 模块名
                record.line().unwrap_or(0),              // 行号
                &record.args()                           // 日志message body内容
            )
        })
        .init();

    // create app state
    let state = new_app_state(&config).await?;

    // Create axum router
    let http_router = set_router(state);

    let address: SocketAddr = format!("0.0.0.0:{}", config.app_port).parse()?;
    info!("http server run on:{}", address.to_string());

    // Create a `TcpListener` using tokio.
    let listener = tokio::net::TcpListener::bind(address).await?;

    // http handler
    let http_handler = tokio::spawn(async move {
        // Run the server with graceful shutdown
        axum::serve(listener, http_router)
            .with_graceful_shutdown(graceful_shutdown(Duration::from_secs(
                config.graceful_wait_time,
            )))
            .await
            .expect("failed to start http service");
    });

    // metrics
    let metrics_server = prometheus_init(config.monitor_port);
    let metrics_handler = tokio::spawn(metrics_server);

    // start http and metrics service
    let _ = tokio::try_join!(http_handler, metrics_handler)
        .expect("failed to start http service and metrics service");
    Ok(())
}
