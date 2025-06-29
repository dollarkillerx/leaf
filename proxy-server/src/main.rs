use anyhow::{anyhow, Result};
use clap::Parser;
use log::{error, info, warn};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

mod crypto;
mod protocol;

use crypto::CryptoManager;
use protocol::{HandshakeRequest, HandshakeResponse, ProxyRequest, ProxyResponse};

#[derive(Parser)]
#[command(name = "proxy-server")]
#[command(about = "Secure proxy server")]
struct Args {
    /// Server listen address
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    listen_addr: Option<String>,

    /// Authentication token
    #[arg(short, long)]
    token: Option<String>,

    /// Encryption key (base64 encoded)
    #[arg(short, long)]
    key: Option<String>,

    /// Generate a new encryption key
    #[arg(long)]
    generate_key: bool,
}

#[derive(Debug)]
struct ClientSession {
    client_id: String,
    session_id: String,
    connected_at: std::time::Instant,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    if args.generate_key {
        let key = CryptoManager::generate_key();
        println!("生成的加密密钥: {}", key);
        return Ok(());
    }

    // 检查必需参数
    let listen_addr = args.listen_addr.ok_or_else(|| anyhow!("缺少 --listen-addr 参数"))?;
    let token = args.token.ok_or_else(|| anyhow!("缺少 --token 参数"))?;
    let key = args.key.ok_or_else(|| anyhow!("缺少 --key 参数"))?;

    // 初始化加密管理器
    let crypto = CryptoManager::new(&key)?;
    
    // 存储活跃的客户端会话
    let sessions: Arc<RwLock<HashMap<String, ClientSession>>> = Arc::new(RwLock::new(HashMap::new()));
    
    let listener = TcpListener::bind(&listen_addr).await?;
    info!("代理服务器启动在 {}", listen_addr);

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!("新连接来自: {}", addr);
                let crypto = crypto.clone();
                let sessions = sessions.clone();
                let token = token.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = handle_client_connection(socket, addr, token, crypto, sessions).await {
                        error!("处理客户端连接时出错: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("接受连接时出错: {}", e);
            }
        }
    }
}

async fn handle_client_connection(
    mut client: TcpStream,
    client_addr: SocketAddr,
    token: String,
    crypto: CryptoManager,
    sessions: Arc<RwLock<HashMap<String, ClientSession>>>,
) -> Result<()> {
    // 处理握手认证
    let session_id = perform_handshake(&mut client, &token, &crypto).await?;
    
    // 存储会话信息
    {
        let mut sessions_write = sessions.write().await;
        sessions_write.insert(
            session_id.clone(),
            ClientSession {
                client_id: client_addr.to_string(),
                session_id: session_id.clone(),
                connected_at: std::time::Instant::now(),
            },
        );
    }
    
    info!("客户端 {} 认证成功，会话 ID: {}", client_addr, session_id);
    
    // 处理代理请求
    let target_addr = receive_proxy_request(&mut client, &crypto).await?;
    
    // 连接到目标服务器
    let mut target = match TcpStream::connect(&target_addr).await {
        Ok(conn) => {
            info!("成功连接到目标服务器: {}", target_addr);
            conn
        }
        Err(e) => {
            error!("连接目标服务器失败: {} - {}", target_addr, e);
            send_proxy_response(&mut client, false, &format!("连接失败: {}", e), &crypto).await?;
            return Err(anyhow!("连接目标服务器失败: {}", e));
        }
    };
    
    // 发送成功响应
    send_proxy_response(&mut client, true, "连接成功", &crypto).await?;
    
    // 开始转发数据
    forward_data(client, target, crypto).await?;
    
    // 清理会话
    {
        let mut sessions_write = sessions.write().await;
        sessions_write.remove(&session_id);
    }
    
    info!("客户端 {} 连接结束", client_addr);
    Ok(())
}

async fn perform_handshake(
    client: &mut TcpStream,
    expected_token: &str,
    crypto: &CryptoManager,
) -> Result<String> {
    // 接收握手请求
    let mut length_buf = [0u8; 4];
    client.read_exact(&mut length_buf).await?;
    let length = u32::from_be_bytes(length_buf) as usize;
    
    let mut request_buf = vec![0u8; length];
    client.read_exact(&mut request_buf).await?;
    
    let decrypted_data = crypto.decrypt(&request_buf)?;
    let handshake: HandshakeRequest = serde_json::from_slice(&decrypted_data)?;
    
    // 验证 token
    if handshake.token != expected_token {
        let response = HandshakeResponse {
            success: false,
            message: "认证失败：无效的 token".to_string(),
            session_id: None,
        };
        
        let response_data = serde_json::to_vec(&response)?;
        let encrypted_response = crypto.encrypt(&response_data)?;
        
        let length = (encrypted_response.len() as u32).to_be_bytes();
        client.write_all(&length).await?;
        client.write_all(&encrypted_response).await?;
        
        return Err(anyhow!("认证失败：无效的 token"));
    }
    
    // 生成会话 ID
    let session_id = uuid::Uuid::new_v4().to_string();
    
    // 发送握手响应
    let response = HandshakeResponse {
        success: true,
        message: "认证成功".to_string(),
        session_id: Some(session_id.clone()),
    };
    
    let response_data = serde_json::to_vec(&response)?;
    let encrypted_response = crypto.encrypt(&response_data)?;
    
    let length = (encrypted_response.len() as u32).to_be_bytes();
    client.write_all(&length).await?;
    client.write_all(&encrypted_response).await?;
    
    Ok(session_id)
}

async fn receive_proxy_request(
    client: &mut TcpStream,
    crypto: &CryptoManager,
) -> Result<String> {
    let mut length_buf = [0u8; 4];
    client.read_exact(&mut length_buf).await?;
    let length = u32::from_be_bytes(length_buf) as usize;
    
    let mut request_buf = vec![0u8; length];
    client.read_exact(&mut request_buf).await?;
    
    let decrypted_data = crypto.decrypt(&request_buf)?;
    let request: ProxyRequest = serde_json::from_slice(&decrypted_data)?;
    
    Ok(request.target_addr)
}

async fn send_proxy_response(
    client: &mut TcpStream,
    success: bool,
    message: &str,
    crypto: &CryptoManager,
) -> Result<()> {
    let response = ProxyResponse {
        success,
        message: message.to_string(),
    };
    
    let response_data = serde_json::to_vec(&response)?;
    let encrypted_response = crypto.encrypt(&response_data)?;
    
    let length = (encrypted_response.len() as u32).to_be_bytes();
    client.write_all(&length).await?;
    client.write_all(&encrypted_response).await?;
    
    Ok(())
}

async fn forward_data(
    mut client: TcpStream,
    mut target: TcpStream,
    crypto: CryptoManager,
) -> Result<()> {
    let (mut client_read, mut client_write) = client.split();
    let (mut target_read, mut target_write) = target.split();
    
    let client_to_target = async {
        let mut buf = [0u8; 8192];
        loop {
            // 读取长度
            let mut length_buf = [0u8; 4];
            if client_read.read_exact(&mut length_buf).await.is_err() {
                break;
            }
            let length = u32::from_be_bytes(length_buf) as usize;
            
            // 读取加密数据
            let mut encrypted_buf = vec![0u8; length];
            if client_read.read_exact(&mut encrypted_buf).await.is_err() {
                break;
            }
            
            // 解密数据
            match crypto.decrypt(&encrypted_buf) {
                Ok(decrypted) => {
                    if target_write.write_all(&decrypted).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
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
            
            // 加密数据
            match crypto.encrypt(&buf[..n]) {
                Ok(encrypted) => {
                    let length = (encrypted.len() as u32).to_be_bytes();
                    if client_write.write_all(&length).await.is_err() {
                        break;
                    }
                    if client_write.write_all(&encrypted).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };
    
    tokio::select! {
        _ = client_to_target => info!("客户端到目标的数据传输完成"),
        _ = target_to_client => info!("目标到客户端的数据传输完成"),
    }
    
    Ok(())
} 