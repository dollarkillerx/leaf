use serde::{Deserialize, Serialize};

/// 握手请求结构体
/// 客户端向服务器发送的初始连接请求
#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeRequest {
    /// 认证令牌，用于验证客户端身份
    pub token: String,
    /// 客户端唯一标识符
    pub client_id: String,
}

/// 握手响应结构体
/// 服务器对客户端握手请求的回复
#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeResponse {
    /// 握手是否成功
    pub success: bool,
    /// 响应消息，包含成功或失败的原因
    pub message: String,
    /// 会话ID，握手成功时提供，用于后续通信
    pub session_id: Option<String>,
}

/// 代理请求结构体
/// 客户端请求代理连接到目标地址
#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyRequest {
    /// 目标地址，格式为 "host:port"
    pub target_addr: String,
}

/// 代理响应结构体
/// 服务器对代理请求的回复
#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyResponse {
    /// 代理连接是否成功建立
    pub success: bool,
    /// 响应消息，包含成功或失败的原因
    pub message: String,
} 