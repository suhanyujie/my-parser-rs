extern crate nom;

use nom::{
    bytes::complete::{tag, take_while_m_n},
    combinator::map_res,
    sequence::tuple,
    IResult,
};

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

// fn markdown(input: &str) -> IResult<&str, &str> {
//     let (input, _) = tag("#")(input)?;
//     let mut title = String::new();
//     let (input, title) = tuple((hex_primary, hex_primary, hex_primary))(input)?;
//     Ok((input, &title))
// }

mod tests {
    use super::*;

    #[test]
    // fn test_markdown() {
    //     assert_eq!(markdown("# MySQL 技术内幕\n* 第一个\n* 第二个"),
    //         Ok((
    //             "",
    //             "# MySQL 技术内幕"
    //         ))
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
