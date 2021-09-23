//! 基于 nom 的 markdown 解析器
//! 参考 https://github.com/Geal/nom/blob/master/examples/json.rs
extern crate nom;

use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_while},
    character::complete::{
        alphanumeric1 as alphanumeric, char, line_ending, multispace1, not_line_ending, one_of,
    },
    combinator::{cut, map, opt, value},
    error::{context, convert_error, ContextError, ErrorKind, ParseError, VerboseError},
    multi::{many1, many_m_n, separated_list0},
    number::complete::double,
    sequence::{delimited, pair, preceded, separated_pair, terminated},
    Err, IResult,
};
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct Title {
    content: String,
    level: u8,
}

#[derive(Debug, PartialEq)]
pub enum MdValue {
    Section(String),
    OneTitle(Title),
    Items(Vec<MdValue>),
    Item(String),
}

// fn title_content<'a>(i: &'a str) -> IResult<&'a str, &'a str> {
//     // context("title_content", preceded(not_line_ending, line_ending))
// }

// fn title_level_info<'a>(i: &'a str) -> IResult<&'a str, Vec<&'a str>> {
//     let chars = "\r\n";
//     // pair(many_m_n(0, 7, tag("#")), multispace1, title_content)

//     // 形如 `take_while` 的 nom 组合子返回一个函数。这个函数是一个解析器，我们可以向其中传递源文本。
//     // take_while(move |c| chars.contains(c))(i)
// }

mod tests {
    use super::*;

    // #[test]
    // fn test_sp1() {
    //     let data = "# 背景\n了解解析器是了解编译器的第一步";
    //     assert_eq!(
    //         title_level_info(data),
    //         Ok((" 背景\n了解解析器是了解编译器的第一步", vec!["#"]))
    //     )
    // }
}
