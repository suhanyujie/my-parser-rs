/*
>* 文章名称：从零编写一个解析器（4）—— MySQL 表结构转结构体的渲染
>* 文章来自：https://github.com/suhanyujie/my-parser-rs
>* 标签：Rust，parser

紧接着上一篇的 sql 解析，当我们把建表 SQL 语句中重要信息解析出来以后，我们就能将其渲染为实际需要的结构，比如：json、go struct 等。


*/

// 字段名
// 类型
// tag。多种 tag 以 ` ` 分隔，如：`gorm:"column:id;type:bigint(20);comment:主键" json:"id" form:"id"`

/// 字符串首字母大写
/// https://stackoverflow.com/questions/38406793/why-is-capitalizing-the-first-letter-of-a-string-so-convoluted-in-rust
pub fn uppercase_first(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// 标识符转为**大**驼峰
pub fn to_small_case_camel(input: &str) -> String {
    let str_arr: Vec<&str> = input.split('_').collect();
    let mut new_arr: Vec<String> = Vec::with_capacity(str_arr.len());
    for item in str_arr {
        new_arr.push(uppercase_first(item));
    }
    return new_arr.join("");
}

/// 标识符转为**小**驼峰
pub fn to_big_case_camel(input: &str) -> String {
    let str_arr: Vec<&str> = input.split('_').collect();
    let mut new_arr: Vec<String> = Vec::with_capacity(str_arr.len());
    for item in str_arr {
        new_arr.push(uppercase_first(item));
    }
    return new_arr.join("");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upper_first() {
        let res = uppercase_first("hello world");
        println!("{}", res);
    }

    #[test]
    fn test_identifier_case_camel() {
        let res = to_big_case_camel("user_name");
        println!("{}", res);
    }
}
