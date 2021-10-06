/*
>* 文章名称：从零编写一个解析器（4）—— MySQL 表结构转结构体的渲染
>* 文章来自：https://github.com/suhanyujie/my-parser-rs
>* 标签：Rust，parser

紧接着上一篇的 sql 解析，当我们把建表 SQL 语句中重要信息解析出来以后，我们就能将其渲染为实际需要的结构，比如：json、go struct 等。


*/

extern crate tera;

use std::collections::HashMap;

use crate::sql1::{DataTypeEnum, OneColumn};
use serde_json::from_value;
use serde_json::to_string;
use serde_json::Error;
use serde_json::Value;
use tera::Context;
use tera::Tera;

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

/// 将字符串转为小驼峰风格
pub fn lowercase_first(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
    }
}

/// 标识符转为**小**驼峰
pub fn to_small_case_camel(input: &str) -> String {
    let str_arr: Vec<&str> = input.split('_').collect();
    let mut new_arr: Vec<String> = Vec::with_capacity(str_arr.len());
    for item in str_arr {
        new_arr.push(lowercase_first(item));
    }
    return new_arr.join("");
}

/// 标识符转为**大**驼峰
pub fn to_big_case_camel(input: &str) -> String {
    let str_arr: Vec<&str> = input.split('_').collect();
    let mut new_arr: Vec<String> = Vec::with_capacity(str_arr.len());
    for item in str_arr {
        new_arr.push(uppercase_first(item));
    }
    return new_arr.join("");
}

pub fn to_big_case_camel_helper(args: &HashMap<String, Value>) -> tera::Result<Value> {
    let str1 = match args.get("word") {
        Some(val) => match from_value::<String>(val.clone()) {
            Ok(v) => to_big_case_camel(&v),
            Err(_) => "".to_string(),
        },
        None => "".to_string(),
    };
    Ok(serde_json::json!(str1))
}

/// tera function 将字符串标识符转为小驼峰风格
pub fn to_small_case_camel_helper(args: &HashMap<String, Value>) -> tera::Result<Value> {
    let str1 = match args.get("word") {
        Some(val) => match from_value::<String>(val.clone()) {
            Ok(v) => to_small_case_camel(&v),
            Err(_) => "".to_string(),
        },
        None => "".to_string(),
    };
    Ok(serde_json::json!(str1))
}

pub fn transfer_type(typ: DataTypeEnum) -> String {
    match typ {
        DataTypeEnum::TinyInt => "tinyint".to_string(),
        DataTypeEnum::SmallInt => "smallint".to_string(),
        DataTypeEnum::Int => "int".to_string(),
        DataTypeEnum::Bigint => "bigint".to_string(),
        DataTypeEnum::VarChar(n) => {
            format!("varchar({})", n)
        }
        DataTypeEnum::DateTime(u32) => "datetime".to_string(),
        DataTypeEnum::Text => "text".to_string(),
        DataTypeEnum::BigText => "bigtext".to_string(),
        DataTypeEnum::Decimal(n) => {
            format!("decimal({})", n)
        }
        _ => "Unknown".to_string(),
    }
}

// 数据库类型对应到结构体中的类型映射 todo
pub fn transfer_type_helper(args: &HashMap<String, Value>) -> tera::Result<Value> {
    let typ = match args.get("typ") {
        Some(val) => match from_value::<DataTypeEnum>(val.clone()) {
            Ok(v) => match v {
                DataTypeEnum::TinyInt => "int".to_string(),
                DataTypeEnum::SmallInt => "int".to_string(),
                DataTypeEnum::Int => "int".to_string(),
                DataTypeEnum::Bigint => "int64".to_string(),
                DataTypeEnum::VarChar(_) => "string".to_string(),
                DataTypeEnum::DateTime(_) => "time.Time".to_string(),
                DataTypeEnum::Text => "string".to_string(),
                DataTypeEnum::BigText => "string".to_string(),
                DataTypeEnum::Decimal(_) => "float64".to_string(),
                _ => "Unknown".to_string(),
            },
            Err(_) => "Unknown".to_string(),
        },
        None => "Unknown".to_string(),
    };
    Ok(serde_json::json!(typ))
}

// 定义一个渲染器
#[derive(Debug)]
struct TypeRender<'a> {
    tera: &'a Tera,
    setting: RenderSetting,
}

/// 渲染设置
#[derive(Debug)]
struct RenderSetting {
    field_name_style: FieldNameStyleEnum,
}

// field name 风格
#[derive(Debug)]
enum FieldNameStyleEnum {
    // 小驼峰
    SmallCaseCamel,
    // 大驼峰
    BigCaseCamel,
    // 下划线风格
    Underline,
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
        assert_eq!(to_big_case_camel("user_name"), "UserName".to_string());
        assert_eq!(to_big_case_camel("Hello_world"), "HelloWorld".to_string());
        assert_eq!(to_big_case_camel("Hello__world"), "HelloWorld".to_string());
        assert_eq!(to_big_case_camel("aaaaa"), "Aaaaa".to_string());
    }

    #[test]
    fn render_demo1() {
        let mut tera = match Tera::new("./data/*.tpl") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.register_function("transfer_type_helper", transfer_type_helper);
        tera.register_function("to_big_case_camel_helper", to_big_case_camel_helper);
        tera.register_function("to_small_case_camel_helper", to_small_case_camel_helper);
        let mut context = Context::new();
        let f1 = OneColumn {
            name: "id".to_string(),
            typ: DataTypeEnum::Bigint,
            comment: "主键".to_string(),
        };
        let field_arr = vec![f1];

        context.insert("field_arr", &field_arr);
        let struct_str = r###"type PpmOrgCustomerTrace struct {
{{field_str}}
}
"###;
        // Id int64 `gorm:"column:id;type:bigint(20);comment:主键" json:"id" form:"id"`
        let type_tpl = r###"{% for field in field_arr %}
    {{to_big_case_camel_helper(word=field.name)}} {{transfer_type_helper(typ=field.typ)}} `json:"{{to_small_case_camel_helper(word=field.name)}}"`
{% endfor %}
"###;
        let field_str_result = tera.render_str(type_tpl, &context);
        let field_str_result_str = field_str_result.unwrap_or("type_tpl render error".to_string());
        context.insert("field_str", field_str_result_str.trim());
        let struct_result = tera.render_str(struct_str, &context);
        if let Ok(res_str) = &struct_result {
            println!("{}", &res_str);
        }
        assert!(&struct_result.is_ok());
    }
}
