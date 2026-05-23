# rs-bff
rs-bff is used for pb protocol or json protocol serialization and deserialization, as well as calling background grpc microservice or go api service.

# BFF 网关从零到一实现过程

## 一、项目概述

本项目是一个基于 Rust 的 BFF（Backend for Frontend）应用网关，核心职责是对外暴露 HTTP API，对内通过 gRPC 协议调用后端微服务，并完成 JSON 与 Protobuf 之间的协议转换。

## 二、技术选型与依赖配置

用户指定的核心依赖如下，全部放入 `Cargo.toml`：

```toml
[dependencies]
tonic = "0.14.6"
prost = "0.14.3"
tonic-prost = "0.14.6"
tokio = { version = "1.52.1", features = ["full"] }
async-trait = "0.1.89"
axum = "0.8.9"
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
```

在 tonic 0.14 生态中，`tonic-prost-build` 负责编译期 proto 代码生成（取代了老版本 `tonic-build` 的 prost 功能），`tonic-prost` 是运行时的 prost codec 实现。因此 `build-dependencies` 中配置：

```toml
[build-dependencies]
tonic-prost-build = "0.14.6"
```

后续逐步引入的依赖：
- `serde_yaml = "0.9.33"`：配置文件解析
- `thiserror = "2"`：错误类型定义
- `log = "0.4.29"`、`env_logger = "0.11.10"`、`chrono = "0.4.44"`：日志系统

## 三、Protobuf 定义与构建脚本

### 3.1 Proto 文件

在 `proto/` 目录下定义两个示例服务：

- `user.proto`：用户服务，包含 `GetUser`、`CreateUser`、`ListUsers` 三个 RPC 方法
- `order.proto`：订单服务，包含 `GetOrder`、`CreateOrder`、`ListUserOrders` 三个 RPC 方法

### 3.2 构建脚本（build.rs）演变

`build.rs` 经历了多轮优化，最终形态如下：

```rust
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = "proto";
    let out_dir = "src/rust_grpc";

    // 1.生成pb代码之前，先删除原来的rs文件
    let _ = fs::remove_dir_all(out_dir);
    fs::create_dir_all(out_dir)?;

    // 2.读取proto文件
    let mut proto_files = Vec::new();
    for entry in fs::read_dir(proto_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("proto") {
            proto_files.push(path.to_string_lossy().into_owned());
        }
    }
    proto_files.sort();

    // 3.生成pb代码，这里指定了pb代码输出目录位置
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(out_dir)
        .compile_protos(&proto_files, &[proto_dir.to_string()])?;

    // 生成mod.rs文件
    let mut mods = String::new();
    for proto in &proto_files {
        let name = Path::new(proto)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("invalid proto file name")?;
        mods.push_str(&format!("pub mod {};\n", name));
    }

    // 4.将模块列表写入mod.rs中
    fs::write(format!("{}/mod.rs", out_dir), mods)?;

    Ok(())
}
```

关键演进点：
- **输出目录固定**：`.out_dir("src/rust_grpc")`，生成的 `.rs` 文件直接落盘到源码目录
- **自动扫描**：不再硬编码 `proto_files` 数组，而是遍历 `proto/` 目录自动收集
- **自动清旧**：编译前先 `remove_dir_all` 再 `create_dir_all`，确保删除协议后不会残留旧模块
- **自动生成 mod.rs**：根据扫描结果动态写入 `pub mod user;`、`pub mod order;`
- **协议生成**: https://github.com/daheige/hello-pb

### 3.3 模块注册

`src/lib.rs` 直接暴露 `rust_grpc` 模块：

```rust
pub mod rust_grpc;
```

`src/rust_grpc/mod.rs` 由 `build.rs` 自动生成，内容为：

```rust
pub mod order;
pub mod user;
```

这样业务代码可以通过 `crate::rust_grpc::user::UserServiceClient` 直接访问生成的 client，无需 `tonic::include_proto!`。

## 四、核心模块实现

### 4.1 错误处理（src/error.rs）

定义统一的 `AppError`，覆盖 gRPC 调用、传输、序列化三类错误：

```rust
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("grpc error: {0}")]
    Grpc(#[from] tonic::Status),
    #[error("grpc transport error: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("internal error: {0}")]
    Internal(String),
}
```

为 `AppError` 实现 `axum::response::IntoResponse`，将 `tonic::Code` 映射为对应的 HTTP 状态码：
- `NotFound` → 404
- `InvalidArgument` → 400
- `Unauthenticated` → 401
- `PermissionDenied` → 403
- `AlreadyExists` → 409
- `Unavailable` → 503
- `DeadlineExceeded` → 504
- 其余 → 500

响应体统一为 JSON 格式：`{"error": "...", "code": 500}`。

### 4.2 gRPC 客户端管理（src/grpc/）

模块组织采用 **实现分离 + `pub use` 重新导出** 的规范：

```
src/grpc/
  mod.rs       // mod client; pub use client::GrpcClientManager;
  client.rs    // 实际实现
```

`GrpcClientManager` 持有 `UserServiceClient` 和 `OrderServiceClient`，通过 `tonic::transport::Channel` 建立连接：

```rust
#[derive(Clone)]
pub struct GrpcClientManager {
    user_client: UserServiceClient<Channel>,
    order_client: OrderServiceClient<Channel>,
}

impl GrpcClientManager {
    pub async fn new(user_addr: &str, order_addr: &str) -> Result<Self, AppError> {
        let user_channel = Channel::from_shared(user_addr.to_string())
            .map_err(|e| AppError::Internal(format!("invalid user service uri: {}", e)))?
            .connect()
            .await?;
        // ...
    }

    pub fn user_client(&self) -> UserServiceClient<Channel> { self.user_client.clone() }
    pub fn order_client(&self) -> OrderServiceClient<Channel> { self.order_client.clone() }
}
```

### 4.3 HTTP API 层（src/api/）

同样采用 **实现分离 + `pub use` 重新导出**：

```
src/api/
  mod.rs       // mod api; pub use api::{AppState, router};
  api.rs       // 实际实现
```

`api.rs` 的核心职责：

1. **定义 DTO**：对外暴露的 JSON 结构体（如 `User`、`Order`、`CreateUserReq` 等）
2. **协议转换**：为 DTO 实现 `From<crate::rust_grpc::xxx::Xxx>`，将 Protobuf 响应转为 JSON
3. **定义路由**：使用 `axum::Router` 挂载 handler
4. **Handler 实现**：接收 JSON 请求 → 构造 Protobuf 请求 → 调用 gRPC → 转换响应 → 返回 JSON

暴露的 HTTP API：
- `GET/POST /api/users`
- `GET /api/users/{id}`
- `POST /api/orders`
- `GET /api/orders/{id}`
- `GET /api/users/{id}/orders`

### 4.4 配置系统（src/config.rs）

从硬编码环境变量演进为 YAML 配置文件驱动。

`app.yaml` 结构：

```yaml
app_debug: true
app_port: 8080
monitor_port: 8090
log_level: "debug"

services:
  - name: user
    target: http://127.0.0.1:50051
  - name: order
    target: http://127.0.0.1:50052
```

`Config` 结构体：

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub app_debug: bool,
    pub app_port: u16,
    pub monitor_port: u16,
    pub log_level: String,
    pub services: Vec<ServiceConfig>,
}
```

提供两个核心方法：
- `from_yaml(path: &str)`：读取并反序列化 YAML
- `get_service_target(name: &str) -> Result<String, AppError>`：按名称查找服务地址，找不到直接返回错误，**禁止设置默认值**

### 4.5 入口（src/main.rs）

最终形态：

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_yaml("app.yaml")?;

    // 基于配置动态设置日志级别
    env_logger::Builder::new()
        .target(env_logger::Target::Stdout)
        .parse_filters(&config.log_level)
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {} {}:{}] {}",
                Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
                record.level(),
                record.module_path().unwrap_or("unnamed"),
                record.line().unwrap_or(0),
                &record.args()
            )
        })
        .init();

    let user_service_addr = config.get_service_target("user")?;
    let order_service_addr = config.get_service_target("order")?;

    let grpc_manager = GrpcClientManager::new(&user_service_addr, &order_service_addr).await?;
    let state = AppState {
        grpc_manager: Arc::new(grpc_manager),
    };

    let app = router(state);

    let bind_addr = format!("0.0.0.0:{}", config.app_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("BFF gateway listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}
```

## 五、关键设计决策与演进

### 5.1 Proto 代码生成方式

最初使用 `tonic::include_proto!("user")` 在 `lib.rs` 中内联包含生成的代码。这种方式的问题是无法直观地查看和调试生成的代码。

演进为：
- `build.rs` 通过 `.out_dir("src/rust_grpc")` 将 `.rs` 文件直接生成到源码树
- `src/rust_grpc/mod.rs` 由 `build.rs` 根据 proto 文件动态生成 `mod` 声明
- `lib.rs` 中只需 `pub mod rust_grpc;`

### 5.2 模块导出规范

为避免 `bff::api::api::AppState` 这种重复嵌套，所有包含子模块的目录统一采用：

```rust
// src/<module>/mod.rs
mod <impl_file>;
pub use <impl_file>::{Type, function};
```

外部调用者始终通过 `bff::api::AppState`、`bff::grpc::GrpcClientManager` 访问，路径扁平清晰。

### 5.3 配置系统演进

- **阶段 1**：`std::env::var` 读取环境变量，带硬编码默认值
- **阶段 2**：引入 `serde_yaml`，从 `app.yaml` 读取
- **阶段 3**：`get_service_target` 返回 `Result`，找不到服务时直接报错退出，**禁止默认值**，确保配置缺失能被及时发现

### 5.4 日志系统演进

- **阶段 1**：`tracing` + `tracing-subscriber`（`EnvFilter` + `fmt::layer`）
- **阶段 2**：替换为 `log` + `env_logger` + `chrono`，原因：项目当前不需要 tracing 的 span、instrument 等高级特性，`log` 生态更轻量

`trace.md` 中保留了完整的 tracing 用法参考，方便后续需要时恢复。

## 六、最终目录结构

```
bff/
├── Cargo.toml
├── build.rs
├── app.yaml
├── trace.md
├── self-code.md
├── proto/
│   ├── user.proto
│   └── order.proto
└── src/
    ├── main.rs
    ├── lib.rs
    ├── config.rs
    ├── error.rs
    ├── grpc/
    │   ├── mod.rs
    │   └── client.rs
    ├── api/
    │   ├── mod.rs
    │   └── api.rs
    └── rust_grpc/
        ├── mod.rs          # build.rs 自动生成
        ├── user.rs         # build.rs 生成
        └── order.rs        # build.rs 生成
```

## 七、运行方式

```bash
cargo run
```

服务默认监听 `0.0.0.0:8080`，gRPC 后端地址通过 `app.yaml` 的 `services` 列表配置。

## 八、可观测性
metrics访问地址：http://localhost:8091/metrics
效果如下：
![metrics.png](metrics.png)

## 客户端和服务端pb协议生成
build.rs用于本地protos协议生成，当然你可以把pb协议进行托管，实现方式：https://github.com/daheige/hello-pb

## axum使用参考
- https://crates.io/crates/axum
- https://github.com/daheige/rs-api
