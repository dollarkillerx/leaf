use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeRequest {
    pub token: String,
    pub client_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub success: bool,
    pub message: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyRequest {
    pub target_addr: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyResponse {
    pub success: bool,
    pub message: String,
} 