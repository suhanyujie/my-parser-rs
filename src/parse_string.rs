/*
>* 文章名称：从零编写一个解析器（2）—— 字符串解析
>* 参考地址：https://github.com/Geal/nom/blob/master/examples/string.rs
>* 文章来自：https://github.com/suhanyujie/my-parser-rs
>* 文章作者：[suhanyujie](https://github.com/suhanyujie)
>* Tips：文章如果有任何错误之处，还请指正，谢谢~
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

### 转义字符
要想支持解析带有转义字符的字符串，我们先看看如何解析转义字符。

首先，当我们书写一个字符串时，如：`let hello_str = "hello \"Nico\"";`，当我们把 `hello_str` 变量打印出来时，它显示的是 `hello "Nico"`，也就是说 `"hello \"Nico\""` 表示的真正含义就是 `hello "Nico"`。
基于此，当我们逐个读取字符时，如果遇到转义符 “\” 时，它所表示的内容实际是它后面的双引号 —— `"`；再比如 `"Hi:\nGreen"`，当我们先从字符串定界符 `"` 开始，然后从 `H` 开始读取，直到遇到 “\” 时，如果后面匹配到的是 `n`，则将其视为换行 —— `\n`，而非将其视为 `n`。它和转义双引号的情况有所不同。
因此可以编写以下代码：

```rust
// `nom_char` is `use nom::character::complete::char as nom_char`
fn parse_escaped<'a>(input: &'a str) -> IResult<&'a str, String> {
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
            return Ok((remain, s));
        }
        Err(err) => {
            return Err(err);
        }
    }
}
```

我们通过 [preceded](https://docs.rs/nom/7.0.0/nom/sequence/fn.preceded.html) 组装解析器，它有两个参数，第一个是用于解析 “\” 的解析器，匹配上了，则将其丢弃，
然后用第二个解析器匹配跟随在 “\” 字符后面的内容。如果其后是 `\`，则表示它实际表示的字符是 `\`；如果 `\` 后的字符是 `"`，则实际表示的字符还是单引号 —— `"`。（代码中 `nom_char` 即 `nom::character::complete::char`）

又到了写测试用例的时间了。

```rust
fn test_parse_escaped() {
    println!("{:?}", parse_escaped("\\n"));
    // 字符串为 `\"`，我们实际声明时，要写成 `\\\"`，实际表示的是 `"`
    assert_eq!(parse_escaped("\\\""), Ok(("", '"'.to_string())));
    // 字符串 `\n`，我们实际声明时，要写成 `\\n`，实际表示的是换行符 —— `\n`
    assert_eq!(parse_escaped("\\n"), Ok(("", '\n'.to_string())));
}
```

嗯，不得不承认，用例中，注释写的很完美。

在一个通用的字符串中，可能存在正常的非转义字符，以及转义字符，因此我们要让解析器能进行正常解析，在遇到转义字符时，调用解析转义字符的解析器，这样就能解析完整的字符串了。
在 nom 中的解决思路是，先解析正常的字符串字面量，遇到转义字符时，需要报错，并返回剩余部分，让另一个解析器（解析转义字符的解析器）尝试。此时我们可以借助 [`nom::branch::alt`](https://docs.rs/nom/7.0.0/nom/branch/fn.alt.html)
它接收一批解析器作为参数，会逐个对输入进行解析，如果遇到错误，则尝试下一个解析器，直到成功解析或者尝试完毕。好了，那我们试试把解析转义字符的解析器合解析普通字符的解析器放入其中，写一个测试用例看看效果：

```rust
/// 一个字符串中有正常的字符串字面量，也可能有特殊的转义字符
/// `nom_map` 即 `nom::combinator::map`
fn parse_normal_or_escaped_str(input: &str) -> IResult<&str, String> {
    alt((
        parse_escaped,
        parse_normal_str1,
    ))(input)
}

// test case
fn test_parse_normal_or_escaped_str() {
    // 字符串 `"\"hello \\\"Nico\\\""` 表示的含义是 `hello "Nico"`。（包含普通字符和转义字符）
    assert_eq!(
        parse_normal_or_escaped_str("\"hello \\\"Nico\\\""),
        Ok(("", "hello \"Nico\"".to_string()))
    );
}
```

运行测试用例，但是编译器报错了：

```
running 1 test
thread 'parse_string::tests::test_parse_normal_or_escaped_str' panicked at 'assertion failed: `(left == right)`
  left: `Err(Error(Error { input: "\\\"Nico\\\"", code: Tag }))`,
 right: `Ok(("", "hello \"Nico\""))`', src/parse_string.rs:326:9
```

可以看到在最后一个用例中出现报错了。看来解析器组合子还是有问题。回到 `parse_normal_or_escaped_str` 的实现：

```
alt((
    parse_escaped,
    parse_normal_str1,
))(input)
```

它允许解析**转义字符**或者解析**普通字符串**，我想我们实际想要的是解析**普通字符**而非“字符串”。要验证我们的猜想很简单，既然它允许解析转义字符或者普通字符串，我们就基于这两种情况测试即可：

```
fn test_parse_normal_or_escaped_str1() {
    // 字符串 `"\\\""` 表示的含义是 `\"`，即双引号 —— `"`。（转义字符）
    assert_eq!(
        parse_normal_or_escaped_str("\\\""),
        Ok(("", '"'.to_string()))
    );
    // 字符串 `"\"hello\""` 的含义是 `hello`（普通字符串）
    let input = "\"hello\"";
    assert_eq!(
        parse_normal_or_escaped_str(input),
        Ok(("", "hello".to_string()))
    );
}
```

是的，测试通过。我们需要想办法解决这个问题。

事实上，开始解析一个字符串后，我们要么遇到的是普通字符，要么是转义字符，只需要在遇到转义字符时解析转义字符；在遇到普通字符时，按普通字符解析即可，因此我们重新实现一个解析普通字符的解析器：

```
fn parse_normal(input: &str) -> IResult<&str, String> {
    let mut parser = is_not("\"\\\n\r\t/");
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

我们可以粗略的将转义字符以外的字符作为普通字符，事实上，还会有如 Unicode 之类的转义字符，但为了简化这篇文章，暂不做处理。
转义字符以外的字符我们可以用 `is_not("\"\\\n\r\t/")` 表示。

我们将新实现的 parse_normal 放入组合子中：

```
fn parse_normal_or_escaped(input: &str) -> IResult<&str, String> {
    alt((
        parse_escaped,
        parse_normal,
    ))(input)
}
```

这下，这个解析器既可以解析转义字符，又可以解析普通字符串。是不是以为要成功了？还不够！

我们知道字符串一般由双引号包裹着，因此我们还需要再组合一次：

```
/// `nom_char` 即 `nom::character::complete::char`
fn parse_normal_or_escaped_str(input: &str) -> IResult<&str, String> {
    delimited(nom_char('"'), parse_normal_or_escaped, nom_char('"'))(input)
}
```

可是这样依然不符合需求，它只能解析双引号包裹的转义字符或双引号包裹的普通字符串。

我们希望解析器能持续地根据输入进行解析，并将结果统一放到一个地方，即将结果打包，我们可以利用 [`nom::multi::fold_many0`](https://docs.rs/nom/7.0.0/nom/multi/fn.fold_many0.html)：

```
/// 解析由双引号包裹的字符串
/// 一个字符串由双引号 `"` 包裹着，其中有普通字符，也可能有特殊的转义字符
fn parse_str_with_escaped_and_combine(input: &str) -> IResult<&str, String> {
    let string_builder = fold_many0(
        parse_normal_or_escaped,
        String::new,
        |mut string, part_res| {
            string += &part_res;
            string
        },
    );
    delimited(nom_char('"'), string_builder, nom_char('"'))(input)
}
```

我们用 `parse_normal_or_escaped` 持续地对输入进行解析，直到解析完成或遇到异常，然后将解析结果统一放到初始化的字符串中。

运行测试用例：

```
fn test_parse_normal_or_escaped_str_and_combine() {
    // 转义字符的解析
    assert_eq!(
        parse_str_with_escaped_and_combine("\"\\\"\""),
        Ok(("", "\"".to_string()))
    );
    assert_eq!(
        parse_str_with_escaped_and_combine("\"hello \""),
        Ok(("", "hello ".to_string()))
    );
    // 字符串 `"\"hello \\\"Nico\\\"\""` 表示的含义是 `hello "Nico"`。（包含普通字符和转义字符）
    assert_eq!(
        parse_str_with_escaped_and_combine("\"hello \\\"Nico\\\"\""),
        Ok(("", "hello \"Nico\"".to_string()))
    );
}
```

太酷了，完全正确。

至此，我们总算完成了小小的字符串解析，它既可以解析普通字符串，还能解析带有转义字符的字符串。别看它简单，它会给我们后续解析 SQL 做好铺垫。

## 参考
* https://github.com/Geal/nom/blob/master/examples/string.rs

*/

use nom::character::complete::char as nom_char;
use nom::combinator::map as nom_map;
use nom::combinator::value;
use nom::multi::fold_many0;
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    sequence::{delimited, preceded},
    IResult,
};
use nom::{error::ErrorKind, Err, Parser};

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

fn parse_escaped<'a>(input: &'a str) -> IResult<&'a str, String> {
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
            return Ok((remain, s));
        }
        Err(err) => {
            return Err(err);
        }
    }
}

fn parse_normal(input: &str) -> IResult<&str, String> {
    let parser = is_not("\'\"\\\n\r\t/");
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

/// 解析转义字符，或者普通字符
fn parse_normal_or_escaped(input: &str) -> IResult<&str, String> {
    alt((parse_escaped, parse_normal))(input)
}

/// 解析由引号包裹的转义字符或者普通字符
/// `nom_char` 即 `nom::character::complete::char`
fn parse_normal_or_escaped_str(input: &str) -> IResult<&str, String> {
    delimited(nom_char('"'), parse_normal_or_escaped, nom_char('"'))(input)
}

/// 解析由双引号包裹的字符串
/// 一个字符串由双引号 `"` 包裹着，其中有普通字符，也可能有特殊的转义字符
pub fn parse_str_with_escaped_and_combine(input: &str) -> IResult<&str, String> {
    let string_builder = fold_many0(
        parse_normal_or_escaped,
        String::new,
        |mut string, part_res| {
            string += &part_res;
            string
        },
    );
    delimited(nom_char('"'), string_builder, nom_char('"'))(input)
}

// to fix 字符串定界符
pub fn parse_str_with_escaped_and_combine_in_single_quote(input: &str) -> IResult<&str, String> {
    let string_builder = fold_many0(
        parse_normal_or_escaped,
        String::new,
        |mut string, part_res| {
            string += &part_res;
            string
        },
    );
    delimited(
        alt((nom_char('\''), nom_char('\"'))),
        string_builder,
        alt((nom_char('\''), nom_char('\"'))),
    )(input)
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
    fn test_parse_escaped() {
        println!("{:?}", parse_escaped("\\n"));
        // 字符串为 `\"`，我们实际声明时，要写成 `\\\"`，实际表示的是 `"`
        assert_eq!(parse_escaped("\\\""), Ok(("", '"'.to_string())));
        // 字符串 `\n`，我们实际声明时，要写成 `\\n`，实际表示的是换行符 —— `\n`
        assert_eq!(parse_escaped("\\n"), Ok(("", '\n'.to_string())));
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

    // bad case
    #[test]
    fn test_parse_normal_or_escaped_str() {
        // 字符串 `"\"hello \\\"Nico\\\""` 表示的含义是 `hello "Nico"`。（包含普通字符和转义字符）
        assert_eq!(
            parse_normal_or_escaped_str("\"hello \\\"Nico\\\""),
            Ok(("", "hello \"Nico\"".to_string()))
        );
    }

    #[test]
    fn test_parse_normal_or_escaped_str1() {
        // 字符串 `"\"\\\"\""` 表示的含义是 `\"`，即双引号 —— `"`。（转义字符）
        assert_eq!(
            parse_normal_or_escaped_str("\"\\\"\""),
            Ok(("", '"'.to_string()))
        );
        // 字符串 `"\"hello\""` 的含义是 `hello`（普通字符串）
        let input = "\"hello\"";
        assert_eq!(
            parse_normal_or_escaped_str(input),
            Ok(("", "hello".to_string()))
        );
    }

    #[test]
    fn test_parse_normal_or_escaped_str2() {
        // 字符串 `"\"\\\"\""` 表示的含义是 `\"`，即双引号 —— `"`。（转义字符）
        assert_eq!(
            parse_normal_or_escaped_str("\"\\\"\""),
            Ok(("", '"'.to_string()))
        );
        // 字符串 `"\"hello\""` 的含义是 `hello`（普通字符串）
        assert_eq!(
            parse_normal_or_escaped_str("\"hello\""),
            Ok(("", "hello".to_string()))
        );
    }

    #[test]
    fn test_parse_normal_or_escaped_str_and_combine() {
        // 转义字符的解析
        assert_eq!(
            parse_str_with_escaped_and_combine("\"\\\"\""),
            Ok(("", "\"".to_string()))
        );
        assert_eq!(
            parse_str_with_escaped_and_combine("\"hello \""),
            Ok(("", "hello ".to_string()))
        );
        // 字符串 `"\"hello \\\"Nico\\\"\""` 表示的含义是 `hello "Nico"`。（包含普通字符和转义字符）
        assert_eq!(
            parse_str_with_escaped_and_combine("\"hello \\\"Nico\\\"\""),
            Ok(("", "hello \"Nico\"".to_string()))
        );
    }
}
