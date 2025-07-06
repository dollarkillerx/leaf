use serde::{Deserialize, Serialize};

/// WebSocket 消息类型枚举
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    /// 握手请求
    Handshake(HandshakeRequest),
    /// 握手响应
    HandshakeResponse(HandshakeResponse),
    /// 代理请求
    ProxyRequest(ProxyRequest),
    /// 代理响应
    ProxyResponse(ProxyResponse),
    /// 数据转发
    Data(Vec<u8>),
    /// 错误消息
    Error(String),
}

/// 握手请求结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeRequest {
    /// 认证令牌，用于验证客户端身份
    pub token: String,
    /// 客户端唯一标识符
    pub client_id: String,
}

/// 握手响应结构体
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
#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyRequest {
    /// 目标地址，格式为 "host:port"
    pub target_addr: String,
}

/// 代理响应结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyResponse {
    /// 代理连接是否成功建立
    pub success: bool,
    /// 响应消息，包含成功或失败的原因
    pub message: String,
} 