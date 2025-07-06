#!/bin/bash

# WebSocket 代理使用示例（明文 ws 版）

echo "=== WebSocket 代理使用示例 (ws) ==="

# 设置变量
TOKEN="my-secret-token"
SERVER_ADDR="127.0.0.1:8080"
CLIENT_ADDR="127.0.0.1:1080"

echo "1. 启动 WebSocket 服务器 (ws)..."
echo "命令: cargo run --bin proxy-ws-server -- --token $TOKEN --listen-addr $SERVER_ADDR"
echo "在另一个终端中运行上述命令"

echo ""
echo "2. 启动 WebSocket 客户端..."
echo "命令: cargo run --bin proxy-ws-client -- --token $TOKEN --server-url ws://$SERVER_ADDR/ws --socks-addr $CLIENT_ADDR"
echo "在另一个终端中运行上述命令"

echo ""
echo "3. 测试代理连接..."
echo "使用 curl 测试:"
echo "curl --socks5 $CLIENT_ADDR http://httpbin.org/ip"

echo ""
echo "=== 注意事项 ==="
echo "- 明文 ws 仅适合内网或开发环境，生产环境请用反向代理（如 nginx/caddy）加 TLS"
echo "- 确保防火墙允许相应端口"
echo "- Token 建议用强密码" 