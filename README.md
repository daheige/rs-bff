# rs-bff

rs-bff 是一个基于 Rust 的 BFF（Backend for Frontend）应用网关，对外暴露 HTTP API，对内通过 gRPC 调用后端微服务，并完成 JSON 与 Protobuf 之间的协议转换。

## 一、项目架构

项目采用分层架构组织代码：

```
src/
├── main.rs              # 入口：初始化配置、日志、路由、metrics、优雅退出
├── rust_grpc/           # build.rs 自动生成的本地 proto 代码
│   ├── mod.rs
│   ├── user.rs
│   └── order.rs
├── infra/               # 基础设施层
│   ├── mod.rs
│   ├── config/          # 配置系统 + AppState
│   │   ├── mod.rs
│   │   ├── app.rs       # AppState 定义
│   │   └── config.rs    # Config 解析
│   └── errors/          # 统一错误类型
│       ├── mod.rs
│       └── error.rs
├── interfaces/          # 接口层（HTTP handler + 路由）
│   ├── mod.rs
│   ├── handler/         # HTTP handler 实现
│   │   ├── mod.rs
│   │   └── greeter.rs
│   └── router/          # axum 路由定义
│       ├── mod.rs
│       └── api.rs
└── providers/           # 提供者层（gRPC 客户端等外部服务连接）
    ├── mod.rs
    ├── provider.rs      # AppState 组装
    └── grpc/
        ├── mod.rs
        ├── client.rs    # GrpcClientManager 实现
        └── readme.md
```

### 核心分层职责

- **infra**：配置解析、错误定义、`AppState` 等基础能力，被上层依赖。
- **providers**：管理外部连接（gRPC client、数据库等），负责组装 `AppState`。
- **interfaces**：处理 HTTP 请求，包含路由注册和 handler 实现，依赖 `AppState` 调用底层服务。

## 二、技术栈

| 用途 | 依赖                                                                                                    |
|------|-------------------------------------------------------------------------------------------------------|
| HTTP Web 框架 | [axum](https://crates.io/crates/axum) 0.8.9                                                           |
| gRPC 客户端/运行时 | [tonic](https://crates.io/crates/tonic) 0.14.6 + tonic-prost 0.14.6                                   |
| Protobuf 序列化 | [prost](https://crates.io/crates/prost) 0.14.3                                                        |
| 异步运行时 | [tokio](https://crates.io/crates/tokio) 1.52.3                                                        |
| 配置/JSON | serde + serde_json + serde_yaml                                                                       |
| 日志 | log + env_logger + chrono                                                                             |
| 错误处理 | [thiserror](https://crates.io/crates/thiserror) 2                                                     |
| 可观测性/Metrics | [autometrics](https://crates.io/crates/autometrics) 3.0.0 + [monitor](https://github.com/rs-god/hera) |
| 优雅退出 | [shutdown](https://github.com/rs-god/hera)                                                            |
| 外部 PB 协议托管 | [hello-pb](https://github.com/daheige/hello-pb)                                                       |

## 三、配置系统

通过 `app.yaml` 驱动：

```yaml
app_debug: true               # 调试模式（线上设为 false）
app_port: 8080                # HTTP 服务端口
monitor_port: 8091            # Prometheus metrics 端口
log_level: info               # 日志级别
graceful_wait_time: 5         # 优雅退出等待时间（秒）
services:
  - name: greeter-svc
    target: http://127.0.0.1:50051
  - name: user
    target: http://127.0.0.1:50052
```

配置读取禁止默认值：找不到服务时直接报错退出，确保配置缺失能被及时发现。

## 四、Protobuf 协议

### 本地 proto（build.rs 生成）

`build.rs` 自动扫描 `proto/` 目录，通过 `tonic-prost-build` 生成代码到 `src/rust_grpc/`，并自动创建 `mod.rs`：

- 编译前清理旧文件，删除协议后不会残留旧模块
- 自动按文件名排序并生成 `pub mod xxx;`

当前本地 proto：
- `proto/user.proto`
- `proto/order.proto`

### 外部 PB 协议托管（推荐）

对于多服务共享的协议，推荐通过独立仓库 + git 依赖托管，避免各服务重复生成和版本不一致。

本项目已接入 [hello-pb](https://github.com/daheige/hello-pb) 作为外部协议依赖：

```toml
hello-pb = { git = "https://github.com/daheige/hello-pb", tag = "v1.0.5" }
```

## 五、gRPC 客户端

`GrpcClientManager` 采用懒加载模式：初始化时不建立连接，首次使用时通过 `tokio::sync::OnceCell` 自动创建并缓存。

```rust
pub struct GrpcClientManager {
    target: TargetServices,
    greeter_client: OnceCell<GreeterClient<Channel>>,
}
```

连接参数配置：

| 参数 | 值 | 说明 |
|------|-----|------|
| `http2_keep_alive_interval` | 30s | HTTP/2 心跳间隔 |
| `keep_alive_timeout` | 20s | 心跳超时（内网建议 10s，公网建议 20s） |
| `keep_alive_while_idle` | true | 保持空闲连接 |
| `timeout` | 30s | 单次 RPC 超时 |
| `connect_timeout` | 10s | 连接建立超时 |

### 设计与权衡

1. **Channel 而非直连**
   - tonic 的 `Endpoint::connect()` 返回 `Channel`，其内部基于 `tower_buffer::Buffer` 在后台任务中维护连接，天然支持 HTTP/2 多路复用。
   - `Channel` 的 `Clone` 成本极低（内部为 `Arc`），因此 handler 中每次取到的 `GreeterClient` 都是独立克隆实例，线程安全且无锁竞争。
   - 相比 `connect` 直连，`Channel` 能精确配置 TLS、超时、并发限制、拦截器、负载均衡策略，扩展性更好。

2. **懒加载（`OnceCell`）而非启动时全量连接**
   - 使用 `tokio::sync::OnceCell` 保证并发安全且仅初始化一次。
   - **收益**：降低启动耗时；避免后端服务未就绪导致 BFF 启动失败；减少长期闲置的无用连接。
   - **代价**：首次请求需承担一次 TCP/TLS 握手 RTT（冷启动）；连接问题会推迟到首次调用时才暴露。

3. **连接保活与超时策略**
   - `keep_alive_while_idle(true)` 配合 30s 心跳 + 20s 超时，维持长连接以减少重复握手开销。
   - `timeout(30s)` 限制单次 RPC，防止后端雪崩级联阻塞；`connect_timeout(10s)` 避免 hung 连接拖死资源。
   - **权衡**：超时阈值需与后端实际 P99 对齐，过短会误杀慢请求，过长则失去熔断意义。

4. **错误分层映射**
   - URI 无效 → `AppError::Internal`（配置错误，返回 HTTP 500）。
   - 网络/连接失败 → `AppError::GrpcTransport`（上游不可用，返回 HTTP 502）。
   - 与 `axum::response::IntoResponse` 联动，自动转换为对应 HTTP 状态码，无需 handler 额外处理。

5. **扩展预留**
   - `TargetServices` 和 `GrpcClientManager` 采用显式字段扩展：新增微服务时，只需在 `TargetServices` 中增加地址字段、在 `GrpcClientManager` 中增加对应的 `OnceCell` 字段即可，无需重构整体结构。

## 六、错误处理

统一错误类型 `AppError`：

```rust
pub enum AppError {
    Grpc(tonic::Status),
    GrpcTransport(tonic::transport::Error),
    Serialization(serde_json::Error),
    Internal(String),
}
```

已实现 `axum::response::IntoResponse`，自动映射 HTTP 状态码：

| gRPC Code | HTTP Status |
|-----------|-------------|
| NotFound | 404 |
| InvalidArgument | 400 |
| Unauthenticated | 401 |
| PermissionDenied | 403 |
| AlreadyExists | 409 |
| Unavailable | 503 |
| DeadlineExceeded | 504 |
| 其他 | 500 |

## 七、HTTP API

当前暴露的路由：

```
GET  /                  -> hello,bff
GET  /api/v1/greeter/say/{name}  -> 调用 greeter gRPC 服务
```

未匹配路由会返回统一的 JSON 错误：

```json
{
  "code": 404,
  "message": "api not found",
  "data": {}
}
```

## 八、可观测性

基于 `autometrics` + `monitor` 自动暴露 Prometheus 指标：

- 访问地址：`http://localhost:8091/metrics`
- 每个 HTTP handler 通过 `#[autometrics(objective = API_SLO)]` 自动采集请求延迟、成功率等指标

效果预览：

![metrics.png](metrics.png)

## 九、运行方式

```bash
# 开发运行
cargo run

# 确保 app.yaml 中配置的 gRPC 后端服务已启动
```

服务默认监听 `0.0.0.0:8080`，metrics 监听 `8091`。

## 十、关键演进记录

1. **架构分层**：从扁平结构演进为 `infra / providers / interfaces` 分层，职责更清晰。
2. **PB 协议托管**：从本地 `build.rs` 生成所有协议，演进为本地 proto + 外部 `hello-pb` git 依赖混合模式。
3. **可观测性**：引入 `autometrics` 自动埋点 + Prometheus exporter，替代手动指标采集。
4. **优雅退出**：引入 `shutdown` 组件，支持 `SIGTERM/SIGINT` 信号平滑关闭 HTTP 和 metrics 服务。
5. **日志系统**：从 `tracing` 演进为 `log + env_logger + chrono`，当前场景更轻量（`trace.md` 保留了 tracing 用法参考）。
6. **配置系统**：从硬编码环境变量演进为 YAML 配置驱动，禁止默认值，缺失即报错。
7. **AppState 下沉**：从 `providers` 层上提到 `infra/config/app.rs`，避免循环依赖，分层更纯粹。
8. **gRPC 懒加载**：从启动时全量连接演进为 `OnceCell` 按需初始化，降低启动耗时与无效连接。

## 组件库

- [axum](https://crates.io/crates/axum)
- [rs-api](https://github.com/daheige/rs-api)
- [hello-pb](https://github.com/daheige/hello-pb)
- [hera (monitor/shutdown)](https://github.com/rs-god/hera)
