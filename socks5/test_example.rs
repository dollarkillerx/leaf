// 这是一个简单的测试示例，展示SOCKS5协议的基本结构
// 要运行这个项目，您需要先安装Rust

fn main() {
    println!("SOCKS5 代理服务器测试示例");
    println!("要运行完整的服务器，请执行以下步骤：");
    println!("1. 安装 Rust: https://rustup.rs/");
    println!("2. 在 socks5 目录中运行: cargo run --release");
    println!("3. 服务器将在 127.0.0.1:1080 启动");
}

// SOCKS5 协议常量
const SOCKS_VERSION: u8 = 0x05;
const NO_AUTHENTICATION: u8 = 0x00;
const CONNECT_COMMAND: u8 = 0x01;
const IPV4_ADDRESS: u8 = 0x01;
const DOMAIN_NAME: u8 = 0x03;
const IPV6_ADDRESS: u8 = 0x04;

// 协议状态码
const SUCCESS: u8 = 0x00;
const GENERAL_FAILURE: u8 = 0x01;
const CONNECTION_NOT_ALLOWED: u8 = 0x02;
const NETWORK_UNREACHABLE: u8 = 0x03;
const HOST_UNREACHABLE: u8 = 0x04;
const CONNECTION_REFUSED: u8 = 0x05;
const TTL_EXPIRED: u8 = 0x06;
const COMMAND_NOT_SUPPORTED: u8 = 0x07;
const ADDRESS_TYPE_NOT_SUPPORTED: u8 = 0x08; 