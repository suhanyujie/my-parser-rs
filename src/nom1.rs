extern crate nom;

use nom::{IResult, bytes::complete::{is_a, is_not, tag, take_while_m_n}, character::complete::char, combinator::map_res, sequence::{delimited, terminated, tuple}};

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
    Ok((input, Color {red, green, blue}))
}

fn md_title_level(input: &str) -> IResult<&str, (&str, &str, &str, &str)> {
    tuple((is_a("#"), is_a(" "), is_not("\n"), is_a("\r\n")))(input)
}

// fn md_title_content(input: &str) -> IResult<&str, &str> {
//     terminated(is_not("\r\n"), "\n")(input)
// }

/// md doc title: 
/// * # title one
/// * ## title two
fn md_title(input: &str) -> IResult<&str, &str> {
   delimited(tag("#"), is_not("\n"), char('\n'))(input)
}

mod tests {
    use super::*;

    #[test]
    fn test_title_level() {
        assert_eq!(
            md_title_level("# MySQL 技术内幕\n* 第一个"),
            Ok(("* 第一个", ("#", " ", "MySQL 技术内幕", "\n")))
        )
    }

    #[test]
    fn test_md_title() {
        assert_eq!(md_title("# MySQL 技术内幕\n* 第一个\n* 第二个"),
            Ok((
                "",
                "# MySQL 技术内幕"
            ))
        );
    }

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
