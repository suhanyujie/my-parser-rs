extern crate nom;

use nom::{
    branch::alt,
    bytes::complete::{is_a, is_not, tag, take_while_m_n},
    character::complete::char,
    combinator::{map_parser, map_res},
    multi::{many0, many1},
    sequence::{delimited, pair, terminated, tuple},
    IResult,
};

use crate::whitespace_char;

#[derive(Debug, PartialEq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

fn from_hex(input: &str) -> Result<u8, std::num::ParseIntError> {
    u8::from_str_radix(input, 16)
}

// fn from_hex(input: &str) -> Result<u8, std::num::ParseIntError> {}

fn hex_primary(input: &str) -> IResult<&str, u8> {
    map_res(take_while_m_n(2, 2, is_hex_digit), from_hex)(input)
}

fn is_hex_digit(c: char) -> bool {
    c.is_digit(16)
}

fn hex_color(input: &str) -> IResult<&str, Color> {
    let (input, _) = tag("#")(input)?;
    let (input, (red, green, blue)) = tuple((hex_primary, hex_primary, hex_primary))(input)?;
    Ok((input, Color { red, green, blue }))
}

fn md_title_level(input: &str) -> IResult<&str, (&str, &str)> {
    tuple((is_a("#"), is_a(" ")))(input)
}

// fn md_title_content(input: &str) -> IResult<&str, &str> {
//     terminated(is_not("\r\n"), "\n")(input)
// }

fn md_title_content_part(input: &str) -> IResult<&str, &str> {
    is_not("\n")(input)
}

fn md_title_code_block(input: &str) -> IResult<&str, &str> {
    delimited(char('`'), is_not("`"), char('`'))(input)
}

fn md_title_content(input: &str) -> IResult<&str, Vec<&str>> {
    // map_parser(md_title_content_part, md_title_code_block)
    many1(alt((md_title_content_part, md_title_code_block)))(input)
}

// fn many_part_content<'a, 'b>(input: &'a str) -> IResult<&'a str, &'b str>
// where
//     'b: 'a,
// {
//     let parser = many1(alt((md_title_content_part, md_title_code_block)));
//     let mut result = String::new();
//     if let Ok((remain, res_list)) = parser(input) {
//         result = res_list.join("");
//     }
//     let res_ref: &'b str = &*result;
//     return Ok(("", res_ref));
// }

// todo
fn md_title_content_dealed(input: &str) -> IResult<&str, Vec<&str>> {
    // map_parser(md_title_content_part, md_title_code_block)
    many1(alt((md_title_content_part, md_title_code_block)))(input)
}

/// md doc title:
/// * # title one
/// * ## title two
// fn md_title(input: &str) -> IResult<&str, &str> {
//     delimited(tag("#"), many1(" "), char('\n'))(input)
// }

mod tests {
    use super::*;

    #[test]
    fn test_md_title_content() {
        let text = "## `MySQL` 技术内幕 `internal`\n* 第一个";
        let mut result = String::new();
        if let Ok(content_info) = md_title_content(text) {
            result = content_info.1.join("");
        }
        println!("the title content is: {}", result);
    }

    #[test]
    fn test_title_level() {
        assert_eq!(
            md_title_level("## MySQL 技术内幕\n* 第一个"),
            Ok(("* 第一个", ("#", " ")))
        );
    }

    // #[test]
    // fn test_md_title() {
    //     assert_eq!(
    //         md_title("# MySQL 技术内幕\n* 第一个\n* 第二个"),
    //         Ok(("", "# MySQL 技术内幕"))
    //     );
    // }

    #[test]
    fn test_from_hex() {
        assert_eq!(
            hex_color("#2F14DF"),
            Ok((
                "",
                Color {
                    red: 47,
                    green: 20,
                    blue: 223,
                }
            ))
        );
    }
}
