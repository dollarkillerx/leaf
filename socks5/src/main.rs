use anyhow::{anyhow, Result};
use bytes::{Buf, BufMut, BytesMut};
use log::{error, info, warn};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const SOCKS_VERSION: u8 = 0x05;
const NO_AUTHENTICATION: u8 = 0x00;
const CONNECT_COMMAND: u8 = 0x01;
const IPV4_ADDRESS: u8 = 0x01;
const DOMAIN_NAME: u8 = 0x03;
const IPV6_ADDRESS: u8 = 0x04;

#[derive(Debug)]
enum AddressType {
    Ipv4(Ipv4Addr, u16),
    Ipv6(Ipv6Addr, u16),
    Domain(String, u16),
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let listener = TcpListener::bind("127.0.0.1:1080").await?;
    info!("SOCKS5 代理服务器启动在 127.0.0.1:1080");

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!("新连接来自: {}", addr);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket).await {
                        error!("处理连接时出错: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("接受连接时出错: {}", e);
            }
        }
    }
}

async fn handle_connection(mut client: TcpStream) -> Result<()> {
    // 处理握手
    handle_handshake(&mut client).await?;
    
    // 处理请求
    let target_addr = handle_request(&mut client).await?;
    
    // 连接到目标服务器
    let mut target = TcpStream::connect(target_addr).await?;
    
    // 发送成功响应
    send_success_response(&mut client).await?;
    
    // 开始转发数据
    forward_data(client, target).await?;
    
    Ok(())
}

async fn handle_handshake(client: &mut TcpStream) -> Result<()> {
    let mut buf = [0u8; 2];
    client.read_exact(&mut buf).await?;
    
    let version = buf[0];
    let nmethods = buf[1];
    
    if version != SOCKS_VERSION {
        return Err(anyhow!("不支持的SOCKS版本: {}", version));
    }
    
    let mut methods = vec![0u8; nmethods as usize];
    client.read_exact(&mut methods).await?;
    
    // 检查是否支持无认证方法
    if !methods.contains(&NO_AUTHENTICATION) {
        // 发送不支持认证方法的响应
        let response = [SOCKS_VERSION, 0xFF];
        client.write_all(&response).await?;
        return Err(anyhow!("客户端不支持无认证方法"));
    }
    
    // 发送选择无认证方法的响应
    let response = [SOCKS_VERSION, NO_AUTHENTICATION];
    client.write_all(&response).await?;
    
    info!("握手成功");
    Ok(())
}

async fn handle_request(client: &mut TcpStream) -> Result<SocketAddr> {
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
            
            // 解析域名
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

async fn send_success_response(client: &mut TcpStream) -> Result<()> {
    // SOCKS5 成功响应格式
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

async fn forward_data(mut client: TcpStream, mut target: TcpStream) -> Result<()> {
    let (mut client_read, mut client_write) = client.split();
    let (mut target_read, mut target_write) = target.split();
    
    let client_to_target = async {
        let mut buf = [0u8; 8192];
        loop {
            let n = match client_read.read(&mut buf).await {
                Ok(n) if n == 0 => break,
                Ok(n) => n,
                Err(_) => break,
            };
            if target_write.write_all(&buf[..n]).await.is_err() {
                break;
            }
        }
    };
    
    let target_to_client = async {
        let mut buf = [0u8; 8192];
        loop {
            let n = match target_read.read(&mut buf).await {
                Ok(n) if n == 0 => break,
                Ok(n) => n,
                Err(_) => break,
            };
            if client_write.write_all(&buf[..n]).await.is_err() {
                break;
            }
        }
    };
    
    tokio::select! {
        _ = client_to_target => info!("客户端到目标的数据传输完成"),
        _ = target_to_client => info!("目标到客户端的数据传输完成"),
    }
    
    Ok(())
}
