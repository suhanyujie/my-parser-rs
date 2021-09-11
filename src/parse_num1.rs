/*
>* 文章名称：从零编写一个解析器（1）—— 解析数字
>* 参考地址：https://github.com/Geal/nom/blob/master/doc/making_a_new_parser_from_scratch.md
>* 文章来自：https://github.com/suhanyujie/my-parser-rs
>* 文章作者：[suhanyujie](https://github.com/suhanyujie)
>* Tips：文章如果有任何错误之处，还请指正，谢谢~
>* 标签：Rust，parser

长久以来，由于我在工作中使用 go 语言，所以时常会遇到需要将 sql 转换为 struct 的需求，虽然在网上能够找到一些将 sql、json 等转换为 struct 的工具，但大都无法配置，要么只支持将 json 转 struct，要么转换后 tag 的风格不符合我所需要的。
基于这种情况，我一直想自己写一套可自定义的转换工具，要想能灵活的转换，先需要对源码字符串（sql 或者 json）进行解析，因此，我们从这里开始，逐步学习如何实现一个解析器，最终的目标是可以灵活的将 sql、json 转换为 go struct。

[nom](https://github.com/Geal/nom)是 Rust 中一个强大的解析器库，而我们就是要基于 nom 对源字符串进行解析。

本文的前半部分**深度**参考 nom 仓库中的一个[文档](https://github.com/Geal/nom/blob/master/doc/making_a_new_parser_from_scratch.md)。

万丈高楼平地起，要想用 nom 写好一个解析器，我们先要对 nom 进行一些了解，因此先从一些小示例开始，主要是通过一些 nom 自带的函数来实现简单的解析。

### 第一次解析

根据 [文档](https://github.com/Geal/nom/blob/master/doc/making_a_new_parser_from_scratch.md) 中的介绍，我们先从解析一个括号中的数字 —— `(12345)` 开始。先定义一个函数签名，它用于把字符串 `(12345)` 解析成数字：

```rust
fn parse_u32(input: &[u8]) -> IResult<&[u8], u32>
```

parse_u32 是函数名称，它接收一个 input  参数，`IResult` 即 `nom::IResult`，是 nom 中常用的结果返回类型。可以通过[文档](https://docs.rs/nom/7.0.0/nom/type.IResult.html)查看其声明和注释：

```
/// Holds the result of parsing functions
///
/// It depends on the input type `I`, the output type `O`, and the error type `E`
/// (by default `(I, nom::ErrorKind)`)
///
/// The `Ok` side is a pair containing the remainder of the input (the part of the data that
/// was not parsed) and the produced value. The `Err` side contains an instance of `nom::Err`.
///
/// Outside of the parsing code, you can use the [Finish::finish] method to convert
/// it to a more common result type
pub type IResult<I, O, E = error::Error<I>> = Result<(I, O), Err<E>>;
```

它的类型由输入、输出的类型和错误类型而定，在返回 Ok 时，它包含了输入的剩余部分以及解析结果；在返回 Err 时，它包含的是 `nom::Err` 类型实例。

基于 nom 的解析器，大都是自下而上构建的，先编写最小的解析单元，然后使用组合子将它们组合到更复杂的解析器中。
nom 中已经提供了很多的基础的[解析单元](https://docs.rs/nom/7.0.0/nom/character/complete/index.html)。利用这些解析单元，我们可以做两种选择：

* 1.解析特定的内容
* 2.组合更上层的解析器

围绕这两点，我们可以先开始尝试 —— 解析 `(12345)`

很明显，我们无法直接用基础的解析器直接解析出 `(12345)` 中的数字部分，因为**基础解析器**解析的内容是比较单调的，比如可以用来解析 `aaa`，`97900` 等这类比较由规律的单元。

既然无法直接解析 `(12345)`，我们就需要手动组合这些基础解析器。基础的解析器大都位于 `nom::*::complete` 下，比如 `nom::bytes::complete::tag`。

`(12345)` 由一个左小括号开始，紧跟着一批数字字符然后是右小括号结束。据此我们可以将其拆分为：
* `(`
* `12345`
* `)`

因此实现返回解析 `(` 的解析器的可以选择 `nom::bytes::complete::tag`
它的函数签名是：

```rust
pub fn tag<T, Input, Error: ParseError<Input>>(
    tag: T
) -> impl Fn(Input) -> IResult<Input, Input, Error>
where
    Input: InputTake + Compare<T>,
    T: InputLength + Clone,
```

可以看到该函数的返回值是 `impl Fn(Input) -> IResult<Input, Input, Error>` —— 即一个闭包。该闭包可以解析源字符串中特定的字符串。
比如你像解析 `(` 开头的字符串（如 `(123)`），则可以写成 `tag("(")`；如果你想解析 `##` 开头的字符串（如 `## someMdTitle`），则可以写成：`tag("##")`。
用一个单元测试试试：

```rust
fn test_tag1() {
    // part 1
    fn my_parser1(s: &str) -> IResult<&str, &str> {
        tag("(")(s)
    }
    let res = my_parser1("(123)");
    assert_eq!(res, Ok(("123)", "(")));
    // part 2
    fn my_parser2(s: &str) -> IResult<&str, &str> {
        tag("##")(s)
    }
    assert_eq!(my_parser2("## someMdTitle"), Ok((" someMdTitle", "##")));
}
```

单元测试中的第 1 部分中，声明了一个解析 `(` 的解析器，然后调用解析器解析字符串：`my_parser1("(123)");`
然后断言，返回 `Ok()`，其中包含的值是一个元组，第 0 个元素是解析完剩余的字符串 `"123)"`，第 1 个值是解析结果 `(`。

单元测试中的第 2 部分中，声明了一个解析 `##` 的解析器，然后调用该解析器解析字符串：`my_parser2("## someMdTitle")`，并断言其返回值
返回 `Ok()`，其中包含的值是一个元组，第 0 个元素是解析完剩余的字符串 ` someMdTitle`，第 1 个值是解析结果 `##`。

好了，虽然有点初级，但至少我们起步了！

## 加速
虽然我们可以通过简单的解析器解析出需要的字符串，但我们的目标可不是单纯地解析出 `(` 或者 `##` 之类地单调字符，我们的首要目标是解析出字符串（`(12345)`）中括号中的数字！

此时，我们就需要用到组合子。通过组合子对不同基础解析器的组合，可以组合出更复杂的解析器。

nom 仓库中提供了一个分类组合子的[文档](https://github.com/Geal/nom/blob/master/doc/choosing_a_combinator.md)。

从其中，我们可以找到一个适用于我们场景的组合子，例如：[delimited](https://docs.rs/nom/7.0.0/nom/sequence/fn.delimited.html)
文档中对该解析器的描述是：用第一个解析器匹配一个对象，然后丢弃它；然后用第二个解析器匹配特定的内容，并获取它；最后用第三个解析器匹配对象，并将对象丢弃。
刚好适用于我们的 `(12345)`

* 1.用第一个解析器解析出 `(`，并丢弃；
* 2.然后用第二个解析器匹配出 `12345`；
* 3.最后用第三个解析器解析出 `)` 并丢弃。

用代码实现如下：

```rust
fn parse_u32(input: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(tag("("), digit0, tag(")"))(input)
}
```

是的，只有一行代码，就能实现解析 `(12345)`。这个函数中，我们可能对 `digit0` 比较陌生，它是 delimited 函数中的第二个解析器，nom 中[封装好的](https://docs.rs/nom/7.0.0/nom/character/complete/fn.digit0.html)。
它位于 `nom::character::complete::digit0`。

## 解析数字
细心的朋友可能注意到，`parse_u32` 函数返回值是 `IResult<&[u8], &[u8]>`，也就是说，在成功时，它返回的输入的剩余数据的类型是 `&[u8]`；解析的结果的类型也是 `&[u8]`，
这是否和我们所说的拿到**数值**不太一致？

是的，文章开头，我们的函数签名是 `fn parse_u32(input: &[u8]) -> IResult<&[u8], u32>`。
我们期望剩余的输入是 `&[u8]`，解析的结果是 `u32` 类型。因此，我们需要在基于 parse_u32 的基础上，将 `&[u8]` 类型的解析结果转换成 String，再将字符串转换成 u32 类型：

```
fn parse_u32_ver1(input: &[u8]) -> IResult<&[u8], u32> {
    let mut my_parser = delimited(tag("("), digit0, tag(")"));
    let res = my_parser(input);
    match res {
        Ok((remain, raw)) => {
            let s1 = String::from_utf8_lossy(raw);
            let num: u32 = s1.parse().unwrap();
            Ok((remain, num))
        }
        Err(err) => Err(err),
    }
}
```

我们通过 `String::from_utf8_lossy(raw);` 将解析结果转成 utf-8 编码的字符串，通过 `let num: u32 = s1.parse().unwrap();` 将字符串转换成 u32 类型。

写个单测验证一下：

```rust
fn test_parse_u32_ver1() {
    assert_eq!(
        parse_u32_ver1("(12345)".as_bytes()),
        Ok(("".as_bytes(), 12345))
    );

    assert_eq!(parse_u32_ver1("(0)".as_bytes()), Ok(("".as_bytes(), 0)));
}
```

太棒了，验证通过！

至此，总算是完成了一个简单的解析器，但距离解析 sql、json 还很远，不着急，慢慢来。我们下一章来试试如何解析字符串。

## 参考
* https://github.com/Geal/nom/blob/master/doc/making_a_new_parser_from_scratch.md

*/

use nom::bytes::complete::tag;
use nom::character::complete::digit0;
use nom::sequence::delimited;
use nom::IResult;

fn parse_u32(input: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(tag("("), digit0, tag(")"))(input)
}

fn parse_u32_ver1(input: &[u8]) -> nom::IResult<&[u8], u32> {
    let mut my_parser = delimited(tag("("), digit0, tag(")"));
    let res = my_parser(input);
    match res {
        Ok((remain, raw)) => {
            let s1 = String::from_utf8_lossy(raw);
            let num: u32 = s1.parse().unwrap();
            Ok((remain, num))
        }
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use nom::bytes::complete::tag;

    use super::*;

    #[test]
    fn test_parse_u32_ver1() {
        assert_eq!(
            parse_u32_ver1("(12345)".as_bytes()),
            Ok(("".as_bytes(), 12345))
        );

        assert_eq!(parse_u32_ver1("(0)".as_bytes()), Ok(("".as_bytes(), 0)));
    }

    #[test]
    fn test_tag1() {
        // part 1
        fn my_parser1(s: &str) -> IResult<&str, &str> {
            tag("(")(s)
        }
        let res = my_parser1("(123)");
        assert_eq!(res, Ok(("123)", "(")));
        // part 2
        fn my_parser2(s: &str) -> IResult<&str, &str> {
            tag("##")(s)
        }
        assert_eq!(my_parser2("## someMdTitle"), Ok((" someMdTitle", "##")));
    }

    #[test]
    fn test_num1() {
        if let Ok((p1, p2)) = parse_u32("(12345)".as_bytes()) {
            let res = String::from_utf8(p2.to_vec());
            if res.is_ok() {
                println!("parse res is: {}", res.unwrap());
            }
        } else {
            eprintln!("error parsing...");
        }
    }
}
