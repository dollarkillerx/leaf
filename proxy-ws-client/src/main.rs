use anyhow::{anyhow, Result};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use clap::Parser;
use futures_util::{sink::SinkExt, stream::StreamExt};
use log::{error, info, warn};
use std::{net::{Ipv4Addr, Ipv6Addr, SocketAddr}, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};
use url::Url;
use uuid::Uuid;

mod protocol;

use protocol::{HandshakeRequest, HandshakeResponse, ProxyRequest, ProxyResponse, WsMessage};

const SOCKS_VERSION: u8 = 0x05;
const NO_AUTHENTICATION: u8 = 0x00;
const CONNECT_COMMAND: u8 = 0x01;
const IPV4_ADDRESS: u8 = 0x01;
const DOMAIN_NAME: u8 = 0x03;
const IPV6_ADDRESS: u8 = 0x04;

#[derive(Parser)]
#[command(name = "proxy-ws-client")]
#[command(about = "WebSocket proxy client with SOCKS5 support")]
struct Args {
    /// SOCKS5 listen address
    #[arg(short = 'l', long, default_value = "127.0.0.1:1080")]
    socks_addr: String,

    /// WebSocket server URL
    #[arg(short = 's', long, default_value = "ws://127.0.0.1:8080/ws")]
    server_url: String,

    /// Authentication token
    #[arg(short, long)]
    token: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    let listener = TcpListener::bind(&args.socks_addr).await?;
    info!("SOCKS5 代理客户端启动在 {}", args.socks_addr);
    info!("连接到 WebSocket 服务器: {}", args.server_url);

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!("新 SOCKS5 连接来自: {}", addr);
                let server_url = args.server_url.clone();
                let token = args.token.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_socks_connection(socket, server_url, token).await {
                        error!("处理 SOCKS5 连接时出错: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("接受 SOCKS5 连接时出错: {}", e);
            }
        }
    }
}

async fn handle_socks_connection(
    mut client: TcpStream,
    server_url: String,
    token: String,
) -> Result<()> {
    // 处理 SOCKS5 握手
    handle_socks_handshake(&mut client).await?;

    // 处理 SOCKS5 请求
    let target_addr = handle_socks_request(&mut client).await?;

    // 连接到 WebSocket 服务器
    let (mut ws_stream, _) = connect_async(&server_url).await?;
    info!("WebSocket 连接建立");

    // 进行握手认证
    let session_id = perform_ws_handshake(&mut ws_stream, &token).await?;

    // 发送代理请求
    let success = send_proxy_request(&server_url, &target_addr, &token).await?;

    if success {
        // 发送 SOCKS5 成功响应
        send_socks_success_response(&mut client).await?;

        // 开始转发数据
        forward_data_via_ws(client, server_url, token, target_addr).await?;
    } else {
        // 发送 SOCKS5 失败响应
        send_socks_failure_response(&mut client).await?;
        return Err(anyhow!("代理服务器连接失败"));
    }

    Ok(())
}

async fn handle_socks_handshake(client: &mut TcpStream) -> Result<()> {
    let mut buf = [0u8; 2];
    client.read_exact(&mut buf).await?;

    let version = buf[0];
    let nmethods = buf[1];

    if version != SOCKS_VERSION {
        return Err(anyhow!("不支持的SOCKS版本: {}", version));
    }

    let mut methods = vec![0u8; nmethods as usize];
    client.read_exact(&mut methods).await?;

    if !methods.contains(&NO_AUTHENTICATION) {
        let response = [SOCKS_VERSION, 0xFF];
        client.write_all(&response).await?;
        return Err(anyhow!("客户端不支持无认证方法"));
    }

    let response = [SOCKS_VERSION, NO_AUTHENTICATION];
    client.write_all(&response).await?;

    info!("SOCKS5 握手成功");
    Ok(())
}

async fn handle_socks_request(client: &mut TcpStream) -> Result<SocketAddr> {
    let mut buf = [0u8; 4];
    client.read_exact(&mut buf).await?;

    let version = buf[0];
    let command = buf[1];
    let _reserved = buf[2];
    let address_type = buf[3];

    if version != SOCKS_VERSION {
        return Err(anyhow!("不支持的SOCKS版本: {}", version));
    }

    if command != CONNECT_COMMAND {
        return Err(anyhow!("不支持的命令: {}", command));
    }

    let target_addr = match address_type {
        IPV4_ADDRESS => {
            let mut addr_buf = [0u8; 4];
            client.read_exact(&mut addr_buf).await?;
            let ip = Ipv4Addr::new(addr_buf[0], addr_buf[1], addr_buf[2], addr_buf[3]);

            let mut port_buf = [0u8; 2];
            client.read_exact(&mut port_buf).await?;
            let port = u16::from_be_bytes(port_buf);

            SocketAddr::from((ip, port))
        }
        DOMAIN_NAME => {
            let mut len_buf = [0u8; 1];
            client.read_exact(&mut len_buf).await?;
            let domain_len = len_buf[0] as usize;

            let mut domain_buf = vec![0u8; domain_len];
            client.read_exact(&mut domain_buf).await?;
            let domain = String::from_utf8(domain_buf)?;

            let mut port_buf = [0u8; 2];
            client.read_exact(&mut port_buf).await?;
            let port = u16::from_be_bytes(port_buf);

            info!("连接到域名: {}:{}", domain, port);

            let addrs = tokio::net::lookup_host(format!("{}:{}", domain, port)).await?;
            addrs.into_iter()
                .next()
                .ok_or_else(|| anyhow!("无法解析域名: {}", domain))?
        }
        IPV6_ADDRESS => {
            let mut addr_buf = [0u8; 16];
            client.read_exact(&mut addr_buf).await?;
            let ip = Ipv6Addr::from(addr_buf);

            let mut port_buf = [0u8; 2];
            client.read_exact(&mut port_buf).await?;
            let port = u16::from_be_bytes(port_buf);

            SocketAddr::from((ip, port))
        }
        _ => return Err(anyhow!("不支持的地址类型: {}", address_type)),
    };

    info!("目标地址: {}", target_addr);
    Ok(target_addr)
}

async fn perform_ws_handshake(
    ws_stream: &mut tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>,
    token: &str,
) -> Result<String> {
    // 发送握手请求
    let handshake = WsMessage::Handshake(HandshakeRequest {
        token: token.to_string(),
        client_id: Uuid::new_v4().to_string(),
    });

    let handshake_text = serde_json::to_string(&handshake)?;
    ws_stream
        .send(TungsteniteMessage::Text(handshake_text))
        .await?;

    // 等待握手响应
    if let Some(Ok(msg)) = ws_stream.next().await {
        if let TungsteniteMessage::Text(text) = msg {
            match serde_json::from_str::<WsMessage>(&text) {
                Ok(WsMessage::HandshakeResponse(response)) => {
                    if response.success {
                        info!("WebSocket 握手成功");
                        Ok(response.session_id.unwrap_or_default())
                    } else {
                        Err(anyhow!("WebSocket 握手失败: {}", response.message))
                    }
                }
                _ => Err(anyhow!("收到无效的握手响应")),
            }
        } else {
            Err(anyhow!("收到非文本握手响应"))
        }
    } else {
        Err(anyhow!("未收到握手响应"))
    }
}

async fn send_proxy_request(
    server_url: &str,
    target_addr: &SocketAddr,
    token: &str,
) -> Result<bool> {
    let (mut ws_stream, _) = connect_async(server_url).await?;

    // 先进行握手
    let _session_id = perform_ws_handshake(&mut ws_stream, token).await?;

    // 发送代理请求
    let proxy_req = WsMessage::ProxyRequest(ProxyRequest {
        target_addr: target_addr.to_string(),
    });

    let proxy_req_text = serde_json::to_string(&proxy_req)?;
    ws_stream
        .send(TungsteniteMessage::Text(proxy_req_text))
        .await?;

    // 等待代理响应
    if let Some(Ok(msg)) = ws_stream.next().await {
        if let TungsteniteMessage::Text(text) = msg {
            match serde_json::from_str::<WsMessage>(&text) {
                Ok(WsMessage::ProxyResponse(response)) => {
                    if response.success {
                        info!("代理连接成功");
                        Ok(true)
                    } else {
                        error!("代理连接失败: {}", response.message);
                        Ok(false)
                    }
                }
                _ => {
                    error!("收到无效的代理响应");
                    Ok(false)
                }
            }
        } else {
            error!("收到非文本代理响应");
            Ok(false)
        }
    } else {
        error!("未收到代理响应");
        Ok(false)
    }
}

async fn send_socks_success_response(client: &mut TcpStream) -> Result<()> {
    // SOCKS5 成功响应格式: [version, status, reserved, address_type, ...]
    let response = [
        SOCKS_VERSION, // version
        0x00,          // status (success)
        0x00,          // reserved
        0x01,          // address_type (IPv4)
        0x00, 0x00, 0x00, 0x00, // IP address (0.0.0.0)
        0x00, 0x00,    // port (0)
    ];
    client.write_all(&response).await?;
    Ok(())
}

async fn send_socks_failure_response(client: &mut TcpStream) -> Result<()> {
    // SOCKS5 失败响应格式: [version, status, reserved, address_type, ...]
    let response = [
        SOCKS_VERSION, // version
        0x01,          // status (general failure)
        0x00,          // reserved
        0x01,          // address_type (IPv4)
        0x00, 0x00, 0x00, 0x00, // IP address (0.0.0.0)
        0x00, 0x00,    // port (0)
    ];
    client.write_all(&response).await?;
    Ok(())
}

async fn forward_data_via_ws(
    client: TcpStream,
    server_url: String,
    token: String,
    target_addr: SocketAddr,
) -> Result<()> {
    let (mut ws_stream, _) = connect_async(&server_url).await?;

    // 先进行握手
    let _session_id = perform_ws_handshake(&mut ws_stream, &token).await?;

    // 发送代理请求
    let proxy_req = WsMessage::ProxyRequest(ProxyRequest {
        target_addr: target_addr.to_string(),
    });

    let proxy_req_text = serde_json::to_string(&proxy_req)?;
    ws_stream
        .send(TungsteniteMessage::Text(proxy_req_text))
        .await?;

    // 等待代理响应
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // 使用 Arc<Mutex<>> 来共享客户端连接
    let client = Arc::new(tokio::sync::Mutex::new(client));

    // 启动双向数据转发
    let client_to_server = {
        let client = client.clone();
        async move {
            let mut buf = [0u8; 4096];
            loop {
                let mut client_guard = client.lock().await;
                match client_guard.read(&mut buf).await {
                    Ok(n) if n > 0 => {
                        let data_msg = WsMessage::Data(buf[..n].to_vec());
                        if let Ok(data_text) = serde_json::to_string(&data_msg) {
                            if let Err(e) = ws_sender.send(TungsteniteMessage::Text(data_text)).await {
                                error!("发送数据到服务器时出错: {}", e);
                                break;
                            }
                        }
                    }
                    Ok(0) => break, // EOF
                    Ok(_) => continue, // 其他情况继续
                    Err(e) => {
                        error!("从客户端读取数据时出错: {}", e);
                        break;
                    }
                }
            }
        }
    };

    let server_to_client = {
        let client = client.clone();
        async move {
            while let Some(Ok(msg)) = ws_receiver.next().await {
                match msg {
                    TungsteniteMessage::Text(text) => {
                        match serde_json::from_str::<WsMessage>(&text) {
                            Ok(WsMessage::Data(data)) => {
                                let mut client_guard = client.lock().await;
                                if let Err(e) = client_guard.write_all(&data).await {
                                    error!("写入数据到客户端时出错: {}", e);
                                    break;
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
                    TungsteniteMessage::Binary(data) => {
                        let mut client_guard = client.lock().await;
                        if let Err(e) = client_guard.write_all(&data).await {
                            error!("写入二进制数据到客户端时出错: {}", e);
                            break;
                        }
                    }
                    TungsteniteMessage::Close(_) => {
                        info!("WebSocket 连接关闭");
                        break;
                    }
                    _ => {}
                }
            }
        }
    };

    // 并发执行双向转发
    tokio::select! {
        _ = client_to_server => info!("客户端到服务器转发结束"),
        _ = server_to_client => info!("服务器到客户端转发结束"),
    }

    Ok(())
}
