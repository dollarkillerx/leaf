#!/bin/bash

# WebSocket代理系统使用示例

echo "=== WebSocket代理系统使用示例 ==="
echo ""

# 检查是否已生成证书
if [ ! -f "server.crt" ] || [ ! -f "server.key" ]; then
    echo "1. 生成自签名证书..."
    ./target/release/proxy-ws-server --generate-cert
    echo ""
fi

echo "2. 启动WebSocket代理服务器（后台运行）..."
echo "   服务器将在端口8080上运行，使用HTTPS模式"
echo "   日志文件: server.log"
echo ""

# 启动服务器
./target/release/proxy-ws-server \
    --token my-secret-token \
    --cert-file server.crt \
    --key-file server.key \
    --listen-addr 0.0.0.0:8080 > server.log 2>&1 &

SERVER_PID=$!
echo "   服务器已启动，PID: $SERVER_PID"
echo ""

# 等待服务器启动
sleep 2

echo "3. 启动WebSocket代理客户端（后台运行）..."
echo "   客户端将在端口1080上提供SOCKS5服务"
echo "   日志文件: client.log"
echo ""

# 启动客户端
./target/release/proxy-ws-client \
    --server-url wss://127.0.0.1:8080/proxy \
    --token my-secret-token \
    --skip-ssl-verify > client.log 2>&1 &

CLIENT_PID=$!
echo "   客户端已启动，PID: $CLIENT_PID"
echo ""

# 等待客户端启动
sleep 2

echo "4. 测试SOCKS5代理连接..."
echo "   使用curl测试代理功能"
echo ""

# 测试连接
echo "测试结果："
curl --socks5 127.0.0.1:1080 --connect-timeout 10 http://httpbin.org/ip 2>/dev/null || echo "连接失败或超时"

echo ""
echo "5. 查看日志..."
echo "   服务器日志: tail -f server.log"
echo "   客户端日志: tail -f client.log"
echo ""

echo "6. 停止服务..."
echo "   停止服务器: kill $SERVER_PID"
echo "   停止客户端: kill $CLIENT_PID"
echo ""

echo "=== 手动测试命令 ==="
echo ""
echo "# 终端1 - 启动服务器"
echo "./target/release/proxy-ws-server --token my-secret-token --cert-file server.crt --key-file server.key"
echo ""
echo "# 终端2 - 启动客户端"
echo "./target/release/proxy-ws-client --server-url wss://127.0.0.1:8080/proxy --token my-secret-token --skip-ssl-verify"
echo ""
echo "# 终端3 - 测试连接"
echo "curl --socks5 127.0.0.1:1080 http://httpbin.org/ip"
echo ""

# 保存PID到文件，方便后续停止
echo $SERVER_PID > server.pid
echo $CLIENT_PID > client.pid

echo "服务已启动，PID保存在 server.pid 和 client.pid 文件中"
echo "使用以下命令停止服务："
echo "  kill \$(cat server.pid) \$(cat client.pid)" 