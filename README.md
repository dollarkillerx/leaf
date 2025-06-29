# leaf
leaf lightweight xray-core manager


# new 
- https://github.com/quinn-rs/quinn

# Leaf Proxy Tool

一个安全的代理工具，包含客户端和服务器组件，支持 SOCKS5 协议和加密通信。

## 功能特性

- **SOCKS5 支持**: 客户端提供本地 SOCKS5 代理服务
- **加密通信**: 使用 AES-GCM 加密客户端和服务器之间的所有通信
- **认证机制**: 基于 token 的客户端认证
- **会话管理**: 服务器端会话跟踪和管理
- **异步处理**: 基于 Tokio 的高性能异步 I/O

## 项目结构

```
leaf/
├── Cargo.toml              # 工作空间配置
├── proxy-client/           # 客户端组件
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # 客户端主程序
│       ├── crypto.rs       # 加密模块
│       └── protocol.rs     # 通信协议
└── proxy-server/           # 服务器组件
    ├── Cargo.toml
    └── src/
        ├── main.rs         # 服务器主程序
        ├── crypto.rs       # 加密模块
        └── protocol.rs     # 通信协议
```

## 快速开始

### 1. 生成加密密钥

```bash
cargo run -p proxy-server -- --generate-key
```

### 2. 启动服务器

```bash
cargo run -p proxy-server -- \
  --listen-addr 0.0.0.0:8080 \
  --token your-secret-token \
  --key your-base64-encoded-key
```

### 3. 启动客户端

```bash
cargo run -p proxy-client -- \
  --socks-addr 127.0.0.1:1080 \
  --server-addr 127.0.0.1:8080 \
  --token your-secret-token \
  --key your-base64-encoded-key
```

### 4. 配置 SOCKS5 代理

将你的应用程序配置为使用 SOCKS5 代理：
- 地址: `127.0.0.1`
- 端口: `1080`

## 命令行参数

### 服务器参数

- `--listen-addr`: 服务器监听地址 (默认: 0.0.0.0:8080)
- `--token`: 认证 token
- `--key`: 加密密钥 (base64 编码)
- `--generate-key`: 生成新的加密密钥

### 客户端参数

- `--socks-addr`: SOCKS5 监听地址 (默认: 127.0.0.1:1080)
- `--server-addr`: 代理服务器地址 (默认: 127.0.0.1:8080)
- `--token`: 认证 token
- `--key`: 加密密钥 (base64 编码)

## 安全特性

- **AES-GCM 加密**: 使用 256 位密钥的 AES-GCM 加密算法
- **随机 Nonce**: 每次加密都使用随机生成的 nonce
- **Token 认证**: 基于预共享 token 的客户端认证
- **会话隔离**: 每个客户端连接都有独立的会话 ID

## 协议说明

### 握手协议

1. 客户端发送 `HandshakeRequest` (包含 token 和 client_id)
2. 服务器验证 token 并返回 `HandshakeResponse` (包含 session_id)

### 代理协议

1. 客户端发送 `ProxyRequest` (包含目标地址)
2. 服务器连接目标并返回 `ProxyResponse`
3. 开始双向数据转发

### 数据格式

所有通信数据都使用以下格式：
- 4 字节长度 (大端序)
- 加密后的数据

## 构建和测试

```bash
# 构建所有组件
cargo build --release

# 运行测试
cargo test

# 运行客户端测试
cargo test -p proxy-client

# 运行服务器测试
cargo test -p proxy-server
```

## 许可证

MIT License
```  
cargo run -p proxy-server -- --generate-key

cargo run -p proxy-server -- --listen-addr 0.0.0.0:8080 --token 1234 --key zBtmakcmTN3R2utmCO4MURnhe29yVR6iqV40S01OgFo=

cargo run -p proxy-client -- --socks-addr 127.0.0.1:1080 --server-addr 127.0.0.1:8080 --token 1234 --key zBtmakcmTN3R2utmCO4MURnhe29yVR6iqV40S01OgFo=

```