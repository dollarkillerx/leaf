use anyhow::{anyhow, Result};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use clap::Parser;
use futures_util::stream::StreamExt;
use log::{error, info, warn};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::Instant,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::RwLock,
};
use uuid::Uuid;

mod protocol;

use protocol::{HandshakeResponse, ProxyResponse, WsMessage};

#[derive(Parser)]
#[command(name = "proxy-ws-server")]
#[command(about = "WebSocket proxy server (ws only)")]
struct Args {
    /// Server listen address
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    listen_addr: String,

    /// Authentication token
    #[arg(short, long)]
    token: String,
}

#[derive(Debug)]
struct ClientSession {
    client_id: String,
    session_id: String,
    connected_at: Instant,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    // 存储活跃的客户端会话
    let sessions: Arc<RwLock<HashMap<String, ClientSession>>> = Arc::new(RwLock::new(HashMap::new()));

    // 创建路由
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state((args.token.clone(), sessions.clone()));

    let addr: SocketAddr = args.listen_addr.parse()?;
    info!("启动 WebSocket 服务器 (ws) 在 {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State((token, sessions)): axum::extract::State<(String, Arc<RwLock<HashMap<String, ClientSession>>>)>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, token, sessions))
}

async fn handle_websocket(
    mut socket: WebSocket,
    token: String,
    sessions: Arc<RwLock<HashMap<String, ClientSession>>>,
) {
    info!("WebSocket 连接建立");

    // 等待握手消息
    if let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            match serde_json::from_str::<WsMessage>(&text) {
                Ok(WsMessage::Handshake(handshake)) => {
                    // 验证 token
                    if handshake.token != token {
                        let response = WsMessage::HandshakeResponse(HandshakeResponse {
                            success: false,
                            message: "认证失败：无效的 token".to_string(),
                            session_id: None,
                        });
                        
                        if let Ok(response_text) = serde_json::to_string(&response) {
                            if let Err(e) = socket.send(Message::Text(response_text.into())).await {
                                error!("发送认证失败响应时出错: {}", e);
                            }
                        }
                        return;
                    }

                    // 生成会话 ID
                    let session_id = Uuid::new_v4().to_string();
                    let client_id = handshake.client_id.clone();
                    // 存储会话信息
                    {
                        let mut sessions_write = sessions.write().await;
                        sessions_write.insert(
                            session_id.clone(),
                            ClientSession {
                                client_id: client_id.clone(),
                                session_id: session_id.clone(),
                                connected_at: Instant::now(),
                            },
                        );
                    }

                    // 发送握手成功响应
                    let response = WsMessage::HandshakeResponse(HandshakeResponse {
                        success: true,
                        message: "认证成功".to_string(),
                        session_id: Some(session_id.clone()),
                    });

                    if let Ok(response_text) = serde_json::to_string(&response) {
                        if let Err(e) = socket.send(Message::Text(response_text.into())).await {
                            error!("发送认证成功响应时出错: {}", e);
                            return;
                        }
                    }

                    info!("客户端 {} 认证成功，会话 ID: {}", client_id, session_id);

                    // 处理后续消息
                    handle_proxy_messages(socket, session_id, sessions).await;
                }
                _ => {
                    error!("收到无效的握手消息");
                    let error_msg = WsMessage::Error("无效的握手消息".to_string());
                    if let Ok(error_text) = serde_json::to_string(&error_msg) {
                        let _ = socket.send(Message::Text(error_text.into())).await;
                    }
                }
            }
        }
    }
}

async fn handle_proxy_messages(
    mut socket: WebSocket,
    session_id: String,
    sessions: Arc<RwLock<HashMap<String, ClientSession>>>,
) {
    let mut target_stream: Option<TcpStream> = None;

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                match serde_json::from_str::<WsMessage>(&text) {
                    Ok(WsMessage::ProxyRequest(proxy_req)) => {
                        // 连接到目标服务器
                        match TcpStream::connect(&proxy_req.target_addr).await {
                            Ok(stream) => {
                                target_stream = Some(stream);
                                let response = WsMessage::ProxyResponse(ProxyResponse {
                                    success: true,
                                    message: "连接成功".to_string(),
                                });
                                
                                if let Ok(response_text) = serde_json::to_string(&response) {
                                    if let Err(e) = socket.send(Message::Text(response_text.into())).await {
                                        error!("发送代理成功响应时出错: {}", e);
                                        break;
                                    }
                                }
                                
                                info!("成功连接到目标服务器: {}", proxy_req.target_addr);
                            }
                            Err(e) => {
                                error!("连接目标服务器失败: {} - {}", proxy_req.target_addr, e);
                                let response = WsMessage::ProxyResponse(ProxyResponse {
                                    success: false,
                                    message: format!("连接失败: {}", e),
                                });
                                
                                if let Ok(response_text) = serde_json::to_string(&response) {
                                    let _ = socket.send(Message::Text(response_text.into())).await;
                                }
                            }
                        }
                    }
                    Ok(WsMessage::Data(data)) => {
                        // 转发数据到目标服务器
                        if let Some(ref mut target) = target_stream {
                            if let Err(e) = target.write_all(&data).await {
                                error!("写入目标服务器时出错: {}", e);
                                break;
                            }
                            
                            // 读取目标服务器的响应并转发回客户端
                            let mut buf = [0u8; 4096];
                            match target.read(&mut buf).await {
                                Ok(n) if n > 0 => {
                                    let data_msg = WsMessage::Data(buf[..n].to_vec());
                                    if let Ok(data_text) = serde_json::to_string(&data_msg) {
                                        if let Err(e) = socket.send(Message::Text(data_text.into())).await {
                                            error!("发送数据到客户端时出错: {}", e);
                                            break;
                                        }
                                    }
                                }
                                Ok(0) => {
                                    info!("目标服务器关闭连接");
                                    break;
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    error!("从目标服务器读取数据时出错: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Ok(WsMessage::Error(error_msg)) => {
                        error!("收到错误消息: {}", error_msg);
                        break;
                    }
                    _ => {
                        warn!("收到未知消息类型");
                    }
                }
            }
            Message::Binary(data) => {
                // 处理二进制数据（直接转发）
                if let Some(ref mut target) = target_stream {
                    if let Err(e) = target.write_all(&data).await {
                        error!("写入目标服务器时出错: {}", e);
                        break;
                    }
                }
            }
            Message::Close(_) => {
                info!("WebSocket 连接关闭");
                break;
            }
            _ => {}
        }
    }

    // 清理会话
    {
        let mut sessions_write = sessions.write().await;
        sessions_write.remove(&session_id);
    }
    
    info!("会话 {} 结束", session_id);
}
