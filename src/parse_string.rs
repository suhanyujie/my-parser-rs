/*
>* 文章名称：从零编写一个解析器（2）—— 字符串解析
>* 参考地址：https://github.com/Geal/nom/blob/master/examples/string.rs
>* 文章来自：https://github.com/suhanyujie/my-parser-rs
>* 标签：Rust，parser

在代码中，我们经常会声明变量、声明字符串，然后编写业务逻辑，然后你是否有想过，编译器是如何读懂你的变量声明，你的代码逻辑。
在这个文章中，我们先从字符串开始，了解如何通过代码来识别代码，解析你所编写的内容是什么。

在 Rust 中，我们可以声明一个字符串：`let s1 = String::from("hello world");`
也可以像这样声明一个字符引用（字符串常量）：`let s1: &'static = "hello world";`
但 Rust 的字符串中比较特殊，首先，Rust 中的字符类型是 Unicode 标量值，其中可以存储所有的 utf-8 字符，[占用 4 字节](https://doc.rust-lang.org/std/primitive.char.html)。

Rust 的字符串是 utf-8 编码，长度可动态增长的类型，它在底层通常是由一些列的字节序列构成，经过一些特定的编码后，就能得到你想要的字符串了。

此外，在 Rust 字符串中，还支持携带转义的 Unicode 字符，如：`String::from("\u{1231}")`
因此，我们先确定好支持的常见情况，以及一些特殊的情况。

### 常见情况
所谓通用的情况，就是最常见的字符串字面量，如：`"hello world"`。通过前面的[实践](https://github.com/suhanyujie/my-parser-rs/blob/master/src/parse_num1.rs)，我们可以很快地写出解析该字符串的解析器：

```rust
fn parse_normal_str1<'a>(input: &'a str) -> IResult<&'a str, String> {
    let mut parser = delimited(tag("\""), is_not("\n\""), tag("\""));
    let res = parser(input);
    match res {
        Ok((remain, result)) => {
            return Ok((remain, result.to_string()));
        }
        Err(err) => {
            return Err(err);
        }
    }
}
```

通过解析器 `parse_normal_str1`，将源字符串解析，返回剩余部分，以及解析出的字符串结果。我们视所有的字符串都是以 `"` 开始，以 `"` 结束。
只需通过 [delimited](https://docs.rs/nom/7.0.0/nom/sequence/fn.delimited.html) 我们可以很容易地将字符串从 `""` 中分离出来。运行一个测试用例：

```
 fn test1() {
    let s = r##""hello world""##;
    assert_eq!(parse_normal_str1(s), Ok(("", "hello world".to_string())));
}
```

一如既往地棒，测试通过：

```
running 1 test
test parse_string::tests::test1 ... ok
```

可是，你是否考虑过类似于 `"hello \"Nico\""` 这种中间带有转义符的字符串？先把它放到测试用例中跑一跑：

```
fn test2() {
    assert_eq!(
        parse_normal_str1(r##""hello \"Nico\"""##),
        Ok(("", "hello \"Nico\"".to_string()))
    );
}
```

做好准备，编译器对我们发起攻击了，给出提示信息：

```
thread 'parse_string::tests::test2' panicked at 'assertion failed: `(left == right)`
  left: `Ok(("Nico\\\"\"", "hello \\"))`,
  right: `Ok(("", "hello \"Nico\""))`', src/parse_string.rs:107:9
```

可以看到，解析器只解析到了转义符，遇到转义符后的 `"` 就停止了，把 `Nico\\\"\"` 当作解析后的剩余部分返回，得到解析结果是 `hello \\`，完全不符合预期。

怎么办？

要想支持解析带有转义字符的字符串，我们先看看如何解析转义字符。

### 转义字符




### 特殊情况

## 参考
* https://github.com/Geal/nom/blob/master/examples/string.rs

*/

use nom::character::complete::char as nom_char;
use nom::combinator::value;
use nom::error::{FromExternalError, ParseError};
use nom::multi::fold_many0;
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    sequence::{delimited, preceded},
    IResult,
};
use nom::{error::ErrorKind, Err, Parser};

use nom::combinator::map as nom_map;

use crate::sql::Literal;

fn parse_normal_str1<'a>(input: &'a str) -> IResult<&'a str, String> {
    let mut parser = delimited(tag("\""), is_not("\n\"\\"), tag("\""));
    let res = parser(input);
    match res {
        Ok((remain, result)) => {
            return Ok((remain, result.to_string()));
        }
        Err(err) => {
            return Err(err);
        }
    }
}

fn parse_escaped_str1<'a>(input: &'a str) -> IResult<&'a str, String> {
    let mut parser = preceded(
        nom_char('\\'),
        // 特殊的转义字符
        alt((
            value('\n', nom_char('n')),
            value('\r', nom_char('r')),
            value('\t', nom_char('t')),
            value('\\', nom_char('\\')),
            value('/', nom_char('/')),
            value('"', nom_char('"')),
        )),
    );
    match parser(input) {
        Ok((remain, res_char)) => {
            let s = res_char.to_string();
            println!("{}", &s);
            return Ok((remain, s));
        }
        Err(err) => {
            return Err(err);
        }
    }
}

/// 一个字符串中有正常的字符串字面量，也可能有特殊的转义字符，也可能有转义的空白字符，要分别对它们做处理。
fn parse_str_with_escaped<'a>(input: &'a str) -> IResult<&'a str, String> {
    alt((
        nom_map(parse_normal_str1, |s| s),
        nom_map(parse_escaped_str1, |s| s),
    ))(input)
}

fn parse_str_with_escaped_and_combine(input: &str) -> IResult<&str, String> {
    let string_builder = fold_many0(
        parse_str_with_escaped,
        String::new,
        |mut string, fragment| {
            string += &fragment;
            string
        },
    );
    delimited(nom_char('"'), string_builder, nom_char('"'))(input)
}

#[derive(Debug, PartialEq, Eq)]
enum Fr<'a> {
    Literal(&'a str),
}

fn demo1() {
    println!("{}", String::from("\u{1231}")); // ሱ
    println!("{}", 'c' as u32);
}

#[cfg(test)]
mod tests {
    use nom::character::{is_alphabetic, is_digit};

    use crate::map;

    use super::*;

    #[test]
    fn test1() {
        let s = r##""hello world""##;
        println!("source is: {}", s);
        assert_eq!(parse_normal_str1(s), Ok(("", "hello world".to_string())));
        assert_eq!(
            parse_normal_str1("\"hello \""),
            Ok(("", "hello ".to_string()))
        );
    }

    #[test]
    fn test2() {
        assert_eq!(
            parse_normal_str1(r##""hello \"Nico\"""##),
            Ok(("", "hello \"Nico\"".to_string()))
        );
    }

    #[test]
    fn test_escaped_str() {
        assert_eq!(parse_escaped_str1(r##"\""##), Ok(("", '"'.to_string())));
        assert_eq!(parse_escaped_str1(r##"\n"##), Ok(("", '\n'.to_string())));
        assert_eq!(parse_escaped_str1("\"\n\""), Ok(("", '\n'.to_string())));
    }

    #[test]
    fn test_enum1() {
        let s1 = "hello";
        // bad case:
        // let mut p: dyn FnMut(&str) -> IResult<&str, Fr> =
        //     nom_map(tag("hello"), |parsed_res: &str| Fr::Literal(parsed_res))(s1);
        // assert_eq!(p(s1), Ok(("", Fr::Literal("hello"))));

        // good case:
        // 该 map 的第二个参数可以是一个闭包，也可以是一个枚举变体。
        let res: IResult<&str, Fr> = nom_map(tag("hello"), Fr::Literal)(s1);
        assert_eq!(res, Ok(("", Fr::Literal("hello"))));
    }

    #[test]
    fn test_parse_str_with_escaped() {
        // `\"` 代表意义就是一个双引号。
        assert_eq!(parse_str_with_escaped("\\\""), Ok(("", "\"".to_string())));
    }

    #[test]
    fn test_parse_str_with_escaped_and_combine() {
        let source = "\"hello \"";
        println!("source is: {}", source);
        assert_eq!(
            parse_str_with_escaped_and_combine(source),
            Ok(("", "hello ".to_string()))
        );
        // `"hello \"Nico\""` 就是 `hello "Nico"`
        // assert_eq!(
        //     parse_str_with_escaped_and_combine("\"hello \\\"Nico\\\"\""),
        //     Ok(("", "hello \"Nico\"".to_string()))
        // );
    }
}
