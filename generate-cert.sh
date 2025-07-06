#!/bin/bash

# 生成自签名证书脚本

echo "生成自签名证书..."

# 生成私钥
openssl genrsa -out server.key 2048

# 生成证书签名请求
openssl req -new -key server.key -out server.csr -subj "/C=CN/ST=Beijing/L=Beijing/O=Proxy/OU=IT/CN=localhost"

# 生成自签名证书
openssl x509 -req -days 365 -in server.csr -signkey server.key -out server.crt

# 清理临时文件
rm server.csr

echo "证书生成完成！"
echo "证书文件: server.crt"
echo "私钥文件: server.key" 