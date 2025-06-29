use anyhow::{anyhow, Result};
use clap::Parser;
use log::{error, info, warn};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

mod crypto;
mod protocol;

use crypto::CryptoManager;
use protocol::{HandshakeRequest, HandshakeResponse, ProxyRequest, ProxyResponse};

const SOCKS_VERSION: u8 = 0x05;
const NO_AUTHENTICATION: u8 = 0x00;
const CONNECT_COMMAND: u8 = 0x01;
const IPV4_ADDRESS: u8 = 0x01;
const DOMAIN_NAME: u8 = 0x03;
const IPV6_ADDRESS: u8 = 0x04;

#[derive(Parser)]
#[command(name = "proxy-client")]
#[command(about = "Secure proxy client with SOCKS5 support")]
struct Args {
    /// SOCKS5 listen address
    #[arg(short = 'l', long, default_value = "127.0.0.1:1080")]
    socks_addr: String,

    /// Proxy server address
    #[arg(short = 's', long, default_value = "127.0.0.1:8080")]
    server_addr: String,

    /// Authentication token
    #[arg(short, long)]
    token: String,

    /// Encryption key (base64 encoded)
    #[arg(short, long)]
    key: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    // 初始化加密管理器
    let crypto = CryptoManager::new(&args.key)?;
    
    let listener = TcpListener::bind(&args.socks_addr).await?;
    info!("SOCKS5 代理客户端启动在 {}", args.socks_addr);
    info!("连接到代理服务器: {}", args.server_addr);

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!("新 SOCKS5 连接来自: {}", addr);
                let crypto = crypto.clone();
                let server_addr = args.server_addr.clone();
                let token = args.token.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = handle_socks_connection(socket, server_addr, token, crypto).await {
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
    server_addr: String,
    token: String,
    crypto: CryptoManager,
) -> Result<()> {
    // 处理 SOCKS5 握手
    handle_socks_handshake(&mut client).await?;
    
    // 处理 SOCKS5 请求
    let target_addr = handle_socks_request(&mut client).await?;
    
    // 连接到代理服务器
    let mut server = TcpStream::connect(&server_addr).await?;
    
    // 与代理服务器进行握手认证
    perform_server_handshake(&mut server, &token, &crypto).await?;
    
    // 发送代理请求
    send_proxy_request(&mut server, target_addr, &crypto).await?;
    
    // 接收代理响应
    let response = receive_proxy_response(&mut server, &crypto).await?;
    
    if response.success {
        // 发送 SOCKS5 成功响应
        send_socks_success_response(&mut client).await?;
        
        // 开始转发数据
        forward_data(client, server, crypto).await?;
    } else {
        // 发送 SOCKS5 失败响应
        send_socks_failure_response(&mut client).await?;
        return Err(anyhow!("代理服务器连接失败: {}", response.message));
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
            addrs.into_iter().next().ok_or_else(|| anyhow!("无法解析域名: {}", domain))?
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

async fn perform_server_handshake(
    server: &mut TcpStream,
    token: &str,
    crypto: &CryptoManager,
) -> Result<()> {
    let handshake = HandshakeRequest {
        token: token.to_string(),
        client_id: uuid::Uuid::new_v4().to_string(),
    };
    
    let handshake_data = serde_json::to_vec(&handshake)?;
    let encrypted_data = crypto.encrypt(&handshake_data)?;
    
    // 发送握手请求
    let length = (encrypted_data.len() as u32).to_be_bytes();
    server.write_all(&length).await?;
    server.write_all(&encrypted_data).await?;
    
    // 接收握手响应
    let mut length_buf = [0u8; 4];
    server.read_exact(&mut length_buf).await?;
    let length = u32::from_be_bytes(length_buf) as usize;
    
    let mut response_buf = vec![0u8; length];
    server.read_exact(&mut response_buf).await?;
    
    let decrypted_data = crypto.decrypt(&response_buf)?;
    let response: HandshakeResponse = serde_json::from_slice(&decrypted_data)?;
    
    if !response.success {
        return Err(anyhow!("服务器握手失败: {}", response.message));
    }
    
    info!("服务器握手成功");
    Ok(())
}

async fn send_proxy_request(
    server: &mut TcpStream,
    target_addr: SocketAddr,
    crypto: &CryptoManager,
) -> Result<()> {
    let request = ProxyRequest {
        target_addr: target_addr.to_string(),
    };
    
    let request_data = serde_json::to_vec(&request)?;
    let encrypted_data = crypto.encrypt(&request_data)?;
    
    let length = (encrypted_data.len() as u32).to_be_bytes();
    server.write_all(&length).await?;
    server.write_all(&encrypted_data).await?;
    
    Ok(())
}

async fn receive_proxy_response(
    server: &mut TcpStream,
    crypto: &CryptoManager,
) -> Result<ProxyResponse> {
    let mut length_buf = [0u8; 4];
    server.read_exact(&mut length_buf).await?;
    let length = u32::from_be_bytes(length_buf) as usize;
    
    let mut response_buf = vec![0u8; length];
    server.read_exact(&mut response_buf).await?;
    
    let decrypted_data = crypto.decrypt(&response_buf)?;
    let response: ProxyResponse = serde_json::from_slice(&decrypted_data)?;
    
    Ok(response)
}

async fn send_socks_success_response(client: &mut TcpStream) -> Result<()> {
    let response = [
        SOCKS_VERSION,  // 版本
        0x00,           // 状态码 (成功)
        0x00,           // 保留字段
        0x01,           // 地址类型 (IPv4)
        0x00, 0x00, 0x00, 0x00,  // IP地址 (0.0.0.0)
        0x00, 0x00,     // 端口 (0)
    ];
    
    client.write_all(&response).await?;
    Ok(())
}

async fn send_socks_failure_response(client: &mut TcpStream) -> Result<()> {
    let response = [
        SOCKS_VERSION,  // 版本
        0x01,           // 状态码 (失败)
        0x00,           // 保留字段
        0x01,           // 地址类型 (IPv4)
        0x00, 0x00, 0x00, 0x00,  // IP地址 (0.0.0.0)
        0x00, 0x00,     // 端口 (0)
    ];
    
    client.write_all(&response).await?;
    Ok(())
}

async fn forward_data(
    mut client: TcpStream,
    mut server: TcpStream,
    crypto: CryptoManager,
) -> Result<()> {
    let (mut client_read, mut client_write) = client.split();
    let (mut server_read, mut server_write) = server.split();
    
    let client_to_server = async {
        let mut buf = [0u8; 8192];
        loop {
            let n = match client_read.read(&mut buf).await {
                Ok(n) if n == 0 => break,
                Ok(n) => n,
                Err(_) => break,
            };
            
            // 加密数据
            match crypto.encrypt(&buf[..n]) {
                Ok(encrypted) => {
                    let length = (encrypted.len() as u32).to_be_bytes();
                    if server_write.write_all(&length).await.is_err() {
                        break;
                    }
                    if server_write.write_all(&encrypted).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };
    
    let server_to_client = async {
        let mut buf = [0u8; 8192];
        loop {
            // 读取长度
            let mut length_buf = [0u8; 4];
            if server_read.read_exact(&mut length_buf).await.is_err() {
                break;
            }
            let length = u32::from_be_bytes(length_buf) as usize;
            
            // 读取加密数据
            let mut encrypted_buf = vec![0u8; length];
            if server_read.read_exact(&mut encrypted_buf).await.is_err() {
                break;
            }
            
            // 解密数据
            match crypto.decrypt(&encrypted_buf) {
                Ok(decrypted) => {
                    if client_write.write_all(&decrypted).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };
    
    tokio::select! {
        _ = client_to_server => info!("客户端到服务器的数据传输完成"),
        _ = server_to_client => info!("服务器到客户端的数据传输完成"),
    }
    
    Ok(())
} 