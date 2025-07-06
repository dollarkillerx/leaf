# WebSocket 代理服务器（明文 ws 版）

这是一个基于 WebSocket (ws) 的代理服务器实现，使用 Rust 2024 和 axum 框架构建。

## 特性

- ✅ 基于 WebSocket 协议通信（ws://）
- ✅ Token 认证机制
- ✅ SOCKS5 客户端支持
- ✅ 异步 I/O 和高性能

## 项目结构

```
proxy-ws-server/     # WebSocket 服务器
├── src/
│   ├── main.rs      # 服务器主程序
│   └── protocol.rs  # 协议定义
└── Cargo.toml

proxy-ws-client/     # WebSocket 客户端
├── src/
│   ├── main.rs      # 客户端主程序
│   └── protocol.rs  # 协议定义
└── Cargo.toml
```

## 快速开始

### 1. 启动服务器

```bash
cargo run --bin proxy-ws-server -- --token my-secret-token --listen-addr 127.0.0.1:8080
```

### 2. 启动客户端

```bash
# 支持 ws:// 或 wss://，推荐 ws://
cargo run --bin proxy-ws-client -- --token my-secret-token --server-url ws://127.0.0.1:8080/ws --socks-addr 127.0.0.1:1080
```

### 3. 测试代理

```bash
curl --socks5 127.0.0.1:1080 http://httpbin.org/ip
```

## 协议说明

### WebSocket 消息格式

所有消息都使用 JSON 格式，包含 `type` 和 `data` 字段：

```json
{
  "type": "Handshake",
  "data": {
    "token": "my-secret-token",
    "client_id": "uuid-string"
  }
}
```

### 消息类型

1. **Handshake**: 客户端认证请求
2. **HandshakeResponse**: 服务器认证响应
3. **ProxyRequest**: 代理连接请求
4. **ProxyResponse**: 代理连接响应
5. **Data**: 数据转发
6. **Error**: 错误消息

### 认证流程

1. 客户端连接到 WebSocket 服务器
2. 客户端发送 Handshake 消息，包含 token
3. 服务器验证 token，返回 HandshakeResponse
4. 认证成功后，客户端可以发送代理请求

### 代理流程

1. 客户端发送 ProxyRequest，指定目标地址
2. 服务器连接到目标地址
3. 服务器返回 ProxyResponse
4. 开始双向数据转发

## 命令行参数

### 服务器参数

- `--listen-addr`: 监听地址 (默认: 0.0.0.0:8080)
- `--token`: 认证令牌 (必需)

### 客户端参数

- `--socks-addr`: SOCKS5 监听地址 (默认: 127.0.0.1:1080)
- `--server-url`: WebSocket 服务器 URL (ws:// 或 wss://)
- `--token`: 认证令牌 (必需)

## 安全说明

- 明文 ws 仅适合内网或开发环境，生产环境请用反向代理（如 nginx/caddy）加 TLS。
- Token 建议用强密码。

## 故障排除

- **连接被拒绝**: 检查防火墙设置和端口是否被占用
- **认证失败**: 检查 token 是否匹配
- **代理失败**: 检查目标服务器是否可达

## 开发说明

- Rust: 2024 edition
- axum: 0.8
- tokio: 1.0
- serde: 1.0

## 许可证

MIT License 