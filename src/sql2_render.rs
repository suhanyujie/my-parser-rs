/*
>* 文章名称：从零编写一个解析器（4）—— MySQL 表结构转结构体的渲染
>* 文章来自：https://github.com/suhanyujie/my-parser-rs
>* 标签：Rust，parser

紧接着上一篇的 sql 解析，当我们把建表 SQL 语句中重要信息解析出来以后，我们就能将其渲染为实际需要的结构，比如：json、go struct 等。在这个文章中，我们将尝试将其渲染成 go struct 的结构。




*/

extern crate tera;

use std::collections::HashMap;

use crate::sql1::{DataTypeEnum, OneColumn};
use serde::Serialize;
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

/// 定义一个渲染器，通过“渲染器”可以将解析好的 create sql 转换成需要的结构和数据。
#[derive(Debug)]
pub struct TypeRender {
    tera: Tera,
    tera_ctx: tera::Context,
    raw_tpl: Option<String>,
    setting: RenderSetting,
}

impl TypeRender {
    pub fn new() -> Self {
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

        let setting = RenderSetting {
            field_name_style: FieldNameStyleEnum::SmallCaseCamel,
            need_json_tag: true,
            need_form_tag: false,
            need_gorm_tag: false,
        };

        let mut ctx = Context::new();
        TypeRender {
            tera,
            setting,
            tera_ctx: ctx,
            raw_tpl: None,
        }
    }

    /// 设置字符串模板
    pub fn set_raw_tpl(&mut self, raw_tpl: String) -> &mut Self {
        self.raw_tpl = Some(raw_tpl);
        self
    }

    /// 设置变量值
    pub fn set_var<V: Serialize + ?Sized, K: Into<String>>(
        &mut self,
        key: K,
        val: &V,
    ) -> &mut Self {
        self.tera_ctx.insert(key, val);
        self
    }

    /// 渲染
    pub fn render(&mut self) -> tera::Result<String> {
        let tpl = self.raw_tpl.as_deref().unwrap_or("");
        return self.tera.render_str(tpl, &self.tera_ctx);
    }
}

impl Default for TypeRender {
    fn default() -> Self {
        Self::new()
    }
}

/// 渲染设置
#[derive(Debug)]
pub struct RenderSetting {
    /// 字段名风格。默认小驼峰 —— SmallCaseCamel
    pub field_name_style: FieldNameStyleEnum,
    /// 是否需要 json tag
    pub need_json_tag: bool,
    /// 是否需要 form tag
    pub need_form_tag: bool,
    /// 是否需要 gorm tag
    pub need_gorm_tag: bool,
}

// field name 风格
#[derive(Debug)]
pub enum FieldNameStyleEnum {
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
    fn test_type_render() {
        let mut tr = TypeRender::new();

        let type_tpl = r###"{% for field in field_arr %}    {{to_big_case_camel_helper(word=field.name)}} {{transfer_type_helper(typ=field.typ)}} `json:"{{to_small_case_camel_helper(word=field.name)}}"` {% endfor %}"###;
        let field_arr = get_test_field_arr();
        let rendered_res = tr
            .set_raw_tpl(type_tpl.to_string())
            .set_var("field_arr", &field_arr)
            .render();
        if let Ok(res_str) = &rendered_res {
            println!("{}", &res_str);
        }
        assert!(&rendered_res.is_ok());
    }

    fn get_test_field_arr() -> Vec<OneColumn> {
        let f1 = OneColumn {
            name: "id".to_string(),
            typ: DataTypeEnum::Bigint,
            comment: "主键".to_string(),
        };
        let field_arr = vec![f1];
        return field_arr;
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
        let field_arr = get_test_field_arr();
        context.insert("field_arr", &field_arr);
        let struct_str = r###"type PpmOrgCustomerTrace struct {
{{field_str}}
}
"###;
        // Id int64 `gorm:"column:id;type:bigint(20);comment:主键" json:"id" form:"id"`
        // 4 个空格表示 field 的缩进
        let type_tpl = r###"{% for field in field_arr %}    {{to_big_case_camel_helper(word=field.name)}} {{transfer_type_helper(typ=field.typ)}} `json:"{{to_small_case_camel_helper(word=field.name)}}"` {% endfor %}"###;
        let field_str_result = tera.render_str(type_tpl, &context);
        let field_str_result_str = field_str_result.unwrap_or("type_tpl render error".to_string());
        context.insert("field_str", &field_str_result_str);
        let struct_result = tera.render_str(struct_str, &context);
        if let Ok(res_str) = &struct_result {
            println!("{}", &res_str);
        }
        assert!(&struct_result.is_ok());
    }

    #[test]
    fn test_demo2() {
        // 从 create sql 到渲染
        let mut tr = TypeRender::new();

        let type_tpl = r###"{% for field in field_arr %}    {{to_big_case_camel_helper(word=field.name)}} {{transfer_type_helper(typ=field.typ)}} `json:"{{to_small_case_camel_helper(word=field.name)}}"` {% endfor %}"###;
        let field_arr = get_test_field_arr();
        let rendered_res = tr
            .set_raw_tpl(type_tpl.to_string())
            .set_var("field_arr", &field_arr)
            .render();
        if let Ok(res_str) = &rendered_res {
            println!("{}", &res_str);
        }
        assert!(&rendered_res.is_ok());
    }
}
