# Tracing 用法参考

本项目曾使用 `tracing` + `tracing-subscriber` 作为日志与追踪方案，以下内容供后续需要恢复或参考时使用。

## 依赖

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## 初始化（基于配置文件）

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

let config = Config::from_yaml("app.yaml")?;

tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::new(&config.log_level))
    .with(tracing_subscriber::fmt::layer())
    .init();
```

## 常用宏

```rust
tracing::info!("BFF gateway listening on {}", addr);
tracing::debug!("debug message");
tracing::warn!("warning message");
tracing::error!("error message");
```

## 与 tower-http 集成

若需要记录 HTTP 请求/响应，可在 `axum::Router` 上添加 `tower_http::trace::TraceLayer`：

```rust
use tower_http::trace::TraceLayer;

let app = router(state)
    .layer(TraceLayer::new_for_http());
```

此时需保留 `tower-http` 的 `trace` feature：

```toml
tower-http = { version = "0.6", features = ["trace"] }
```
