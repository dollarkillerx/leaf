use std::collections::HashMap;
use std::io::{stdin, stdout, Write};

/// 用户结构体 - 存储用户的基本信息和账户余额
/// 使用 #[derive(Debug)] 让结构体可以打印调试信息
#[derive(Debug)]
struct User {
    email: String,      // 邮箱作为唯一标识符
    password: String,   // 密码用于身份验证
    balance: f64,       // 账户余额，使用 f64 支持小数
}

/// 为 User 结构体实现方法
/// 这样做的好处：
/// 1. 将相关的功能封装在一起，提高代码组织性
/// 2. 可以添加业务逻辑验证，确保数据一致性
/// 3. 提供清晰的接口，隐藏内部实现细节
impl User {
    /// 构造函数 - 创建新用户
    /// 使用关联函数而不是普通函数的好处：
    /// 1. 更符合面向对象的设计模式
    /// 2. 代码更清晰，明确表示这是创建 User 的方法
    /// 3. 可以在这里添加初始化逻辑
    fn new(email: String, password: String) -> Self {
        User {
            email,
            password,
            balance: 0.0,  // 新用户余额初始化为 0
        }
    }

    /// 显示余额的方法
    /// 使用 &self 表示这是一个只读方法，不会修改用户数据
    /// 格式化输出保留两位小数，提供更好的用户体验
    fn display_balance(&self) {
        println!("当前余额: {:.2}", self.balance);
    }

    /// 存款方法
    /// 返回 Result<(), String> 的好处：
    /// 1. 可以处理错误情况（如负数存款）
    /// 2. 调用者可以决定如何处理错误
    /// 3. 符合 Rust 的错误处理惯例
    fn deposit(&mut self, amount: f64) -> Result<(), String> {
        // 业务逻辑验证：存款金额必须大于0
        if amount <= 0.0 {
            return Err("存款金额必须大于0".to_string());
        }
        self.balance += amount;
        println!("存款成功，当前余额: {:.2}", self.balance);
        Ok(())
    }

    /// 取款方法
    /// 包含两个验证：
    /// 1. 取款金额必须大于0
    /// 2. 取款金额不能超过余额
    fn withdraw(&mut self, amount: f64) -> Result<(), String> {
        if amount <= 0.0 {
            return Err("取款金额必须大于0".to_string());
        }
        if amount > self.balance {
            return Err("余额不足".to_string());
        }
        self.balance -= amount;
        println!("取款成功，当前余额: {:.2}", self.balance);
        Ok(())
    }

    /// 转账方法
    /// 注意：这个方法在实际使用中会遇到借用检查器问题
    /// 因为需要同时修改两个用户的数据
    /// 所以在 user_menu 中我们使用了不同的实现方式
    fn transfer(&mut self, amount: f64, target_user: &mut User) -> Result<(), String> {
        if amount <= 0.0 {
            return Err("转账金额必须大于0".to_string());
        }
        if amount > self.balance {
            return Err("余额不足".to_string());
        }
        
        self.balance -= amount;
        target_user.balance += amount;
        println!("转账成功，当前余额: {:.2}", self.balance);
        Ok(())
    }
}

/// 读取用户输入的辅助函数
/// 为什么要封装成函数：
/// 1. 避免代码重复
/// 2. 统一输入处理逻辑
/// 3. 可以在这里添加输入验证
/// 4. 使用 stdout().flush() 确保提示信息立即显示
fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    stdout().flush().expect("Failed to flush stdout");  // 确保提示立即显示
    let mut input = String::new();
    stdin().read_line(&mut input).expect("Failed to read line");
    input.trim().to_string()  // 去除首尾空白字符
}

/// 读取金额的辅助函数
/// 返回 Result<f64, String> 的好处：
/// 1. 可以处理解析失败的情况
/// 2. 提供有意义的错误信息
/// 3. 调用者可以决定如何处理错误
fn read_amount(prompt: &str) -> Result<f64, String> {
    let input = read_input(prompt);
    input.parse::<f64>().map_err(|_| "请输入正确的金额".to_string())
}

/// 用户注册功能
/// 为什么要传递 &mut HashMap：
/// 1. 需要修改 HashMap 来添加新用户
/// 2. 使用引用避免所有权转移
/// 3. 使用 mut 表示需要修改数据
fn register_user(db_map: &mut HashMap<String, User>) {
    println!("\n=== 用户注册 ===");
    
    let email = read_input("请输入邮箱: ");
    // 检查邮箱是否已被注册
    if db_map.contains_key(&email) {
        println!("该邮箱已被注册");
        return;
    }
    
    let password = read_input("请输入密码: ");
    if password.is_empty() {
        println!("密码不能为空");
        return;
    }
    
    // 使用构造函数创建新用户
    let user = User::new(email.clone(), password);
    db_map.insert(email, user);
    println!("注册成功！");
}

/// 用户登录功能
/// 登录成功后直接进入用户菜单
/// 这样设计的好处：
/// 1. 用户体验更流畅
/// 2. 避免重复的菜单选择
fn login_user(db_map: &mut HashMap<String, User>) {
    println!("\n=== 用户登录 ===");
    
    let email = read_input("请输入邮箱: ");
    let password = read_input("请输入密码: ");
    
    // 使用 if let 进行模式匹配，更简洁
    if let Some(user) = db_map.get(&email) {
        if user.password == password {
            println!("登录成功！");
            user_menu(db_map, email);
        } else {
            println!("密码错误");
        }
    } else {
        println!("用户不存在");
    }
}

/// 用户菜单功能
/// 为什么要传递 email 参数：
/// 1. 需要知道当前登录的是哪个用户
/// 2. 用于在 HashMap 中查找和修改用户数据
/// 3. 避免全局变量的使用
fn user_menu(db_map: &mut HashMap<String, User>, email: String) {
    loop {
        println!("\n=== 用户菜单 ===");
        println!("1. 查看余额");
        println!("2. 存款");
        println!("3. 取款");
        println!("4. 转账");
        println!("5. 退出");
        println!("================");
        
        let choice = read_input("请选择操作: ");
        
        match choice.as_str() {
            "1" => {
                // 查看余额只需要只读访问
                if let Some(user) = db_map.get(&email) {
                    user.display_balance();
                }
            }
            "2" => {
                // 存款需要可变访问来修改余额
                if let Some(user) = db_map.get_mut(&email) {
                    match read_amount("请输入存款金额: ") {
                        Ok(amount) => {
                            if let Err(e) = user.deposit(amount) {
                                println!("错误: {}", e);
                            }
                        }
                        Err(e) => println!("错误: {}", e),
                    }
                }
            }
            "3" => {
                // 取款需要可变访问来修改余额
                if let Some(user) = db_map.get_mut(&email) {
                    match read_amount("请输入取款金额: ") {
                        Ok(amount) => {
                            if let Err(e) = user.withdraw(amount) {
                                println!("错误: {}", e);
                            }
                        }
                        Err(e) => println!("错误: {}", e),
                    }
                }
            }
            "4" => {
                // 转账功能 - 这里展示了如何处理借用检查器问题
                // 问题：需要同时修改两个用户的数据，但 Rust 不允许同时拥有两个可变引用
                // 解决方案：分步骤处理，先读取数据，再分别修改
                match read_amount("请输入转账金额: ") {
                    Ok(amount) => {
                        let target_email = read_input("请输入转账目标邮箱: ");
                        
                        // 检查目标用户是否存在
                        if !db_map.contains_key(&target_email) {
                            println!("目标用户不存在");
                            continue;
                        }
                        
                        // 先检查源用户余额（只读访问）
                        if let Some(user) = db_map.get(&email) {
                            if amount > user.balance {
                                println!("余额不足");
                                continue;
                            }
                        }
                        
                        // 使用分步骤的方式避免借用冲突
                        // 1. 先获取两个用户的当前余额
                        if let (Some(source_balance), Some(target_balance)) = (
                            db_map.get(&email).map(|u| u.balance),
                            db_map.get(&target_email).map(|u| u.balance)
                        ) {
                            // 2. 分别更新两个用户的余额
                            if let Some(user) = db_map.get_mut(&email) {
                                user.balance = source_balance - amount;
                            }
                            
                            if let Some(user) = db_map.get_mut(&target_email) {
                                user.balance = target_balance + amount;
                            }
                            
                            println!("转账成功！");
                            if let Some(user) = db_map.get(&email) {
                                println!("当前余额: {:.2}", user.balance);
                            }
                        }
                    }
                    Err(e) => println!("错误: {}", e),
                }
            }
            "5" => {
                println!("退出用户菜单");
                break;
            }
            _ => {
                println!("无效的选择，请重新输入");
            }
        }
    }
}

/// 显示所有账户信息
/// 使用只读引用 &HashMap，因为我们只需要读取数据
/// 这样设计的好处：
/// 1. 明确表示这个函数不会修改数据
/// 2. 允许多个只读引用同时存在
/// 3. 提高代码的安全性
fn display_all_accounts(db_map: &HashMap<String, User>) {
    println!("\n=== 所有账户信息 ===");
    if db_map.is_empty() {
        println!("暂无账户");
        return;
    }
    
    // 遍历 HashMap 显示所有用户信息
    for (email, user) in db_map {
        println!("邮箱: {}, 余额: {:.2}", email, user.balance);
    }
}

/// 主函数 - 程序的入口点
/// 使用 loop 创建无限循环，直到用户选择退出
/// 这样设计的好处：
/// 1. 程序不会意外退出
/// 2. 用户可以连续进行多个操作
/// 3. 提供清晰的退出选项
fn main() {
    // 使用 HashMap 存储用户数据
    // 为什么选择 HashMap：
    // 1. 快速查找（O(1) 时间复杂度）
    // 2. 使用邮箱作为键，确保唯一性
    // 3. 内存效率高
    let mut db_map: HashMap<String, User> = HashMap::new();

    loop {
        println!("\n=== 银行管理系统 ===");
        println!("1. 注册");
        println!("2. 登录");
        println!("3. 查看所有账户");
        println!("4. 退出");
        println!("==================");

        let choice = read_input("请选择操作: ");

        match choice.as_str() {
            "1" => register_user(&mut db_map),
            "2" => login_user(&mut db_map),
            "3" => display_all_accounts(&db_map),
            "4" => {
                println!("感谢使用，再见！");
                break;
            }
            _ => {
                println!("无效的选择，请重新输入");
            }
        }
    }
}
