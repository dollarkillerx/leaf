
fn ince(x: &mut i32) {
    *x += 1;
}

fn main() {
//  test2();
test3();
}


enum IpAddrKind {
    V4(String),
    V6(String),
}

impl IpAddrKind {
    fn new(kind: &str) -> Self {
        if kind == "V4" {
            Self::V4(String::from("127.0.0.1"))
        } else {
            Self::V6(String::from("::1"))
        }
    }
}
fn test3() {
}

fn test2() {
    let s = String::from("hello world");
    let word = first_word(&s);
    println!("{}",word);
}

fn first_word(s: &String) -> i32 {
    let bytes = s.as_bytes();
    for (i,&item) in bytes.iter().enumerate() {
        if item == b' ' {   
            return i as i32;
        }
    }
    s.len() as i32
}

fn test1() {
    let mut v: Vec<i32> = vec![1,2,3,4,5];
    let b = &v[2];
    println!("{}",b);
    v.push(6);
    println!("{:?}", v);


    let mut x = Box::new(1);
    let y = &mut x;
    **y += 1;  // 需要两次解引用：*y 得到 Box<i32>，**y 得到 i32
    println!("y: {}", y);
    println!("x: {}", x);

    let mut x = 1;
    ince(&mut x);
    println!("x: {}", x);


    // let mut s = String::from("hello");
    // let s2 = &s;
    // let s3 = &mut s;
    // *s3 += " world";
    // println!("{}",s2);
}