mod markdown;
mod redis;
mod nom1;
mod http;
mod json;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Element {
    name: String,
    attributes: Vec<(String, String)>,
    children: Vec<Element>,
}

/// 解析出一个字母 `a`
fn the_letter_a(input: &str) -> Result<(&str, ()), &str> {
    match input.chars().next() {
        Some('a') => Ok((&input['a'.len_utf8()..], ())),
        _ => Err(input),
    }
}

/// 解析一串字符串
fn match_literal(expected: &'static str) -> impl Fn(&str) -> Result<(&str, ()), &str> {
    move |input| match input.get(0..expected.len()) {
        Some(next) if next == expected => Ok((&input[expected.len()..], ())),
        _ => Err(input),
    }
}

/// 识别标识符 v1
fn identifier_v1(input: &str) -> Result<(&str, String), &str> {
    let mut matched = String::new();
    let mut chars = input.chars();
    match chars.next() {
        Some(next) if next.is_alphabetic() => matched.push(next),
        _ => return Err(input),
    }
    while let Some(next) = chars.next() {
        if next.is_alphanumeric() || next == '-' {
            matched.push(next);
        } else {
            break;
        }
    }
    let next_index = matched.len();
    Ok((&input[next_index..], matched))
}

/// 解析器组合器 1
fn pair_v1<P1, P2, R1, R2>(
    parser1: P1,
    parser2: P2,
) -> impl Fn(&str) -> Result<(&str, (R1, R2)), &str>
where
    P1: Fn(&str) -> Result<(&str, R1), &str>,
    P2: Fn(&str) -> Result<(&str, R2), &str>,
{
    move |input| match parser1(input) {
        Ok((next_input, result1)) => match parser2(next_input) {
            Ok((final_result, result2)) => Ok((final_result, (result1, result2))),
            Err(err) => Err(err),
        },
        Err(err) => Err(err),
    }
}

/// 类型转换 v1 版本（旧版本）的 map 实现
fn map_v1<P, F, A, B>(parser: P, map_fn: F) -> impl Fn(&str) -> Result<(&str, B), &str>
where
    P: Fn(&str) -> Result<(&str, A), &str>,
    F: Fn(A) -> B,
{
    move |input| parser(input).map(|(next_input, result)| (next_input, map_fn(result)))
}

/// 类型声明
type ParseResult<'a, Output> = Result<(&'a str, Output), &'a str>;

trait ParserV1<'a, Output> {
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output>;
}

trait Parser<'a, Output> {
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output>;

    fn map<F, NewOutput>(self, map_fn: F) -> BoxedParser<'a, NewOutput>
    where
        Self: Sized + 'a,
        Output: 'a,
        NewOutput: 'a,
        F: Fn(Output) -> NewOutput + 'a,
    {
        BoxedParser::new(map(self, map_fn))
    }

    fn pred<F>(self, pred_fn: F) -> BoxedParser<'a, Output>
    where
        Self: Sized + 'a,
        Output: 'a,
        F: Fn(&Output) -> bool + 'a,
    {
        BoxedParser::new(pred(self, pred_fn))
    }

    fn and_then<F, NextParser, NewOutput>(self, f: F) -> BoxedParser<'a, NewOutput>
    where
        Self: Sized + 'a,
        Output: 'a,
        NewOutput: 'a,
        NextParser: Parser<'a, NewOutput> + 'a,
        F: Fn(Output) -> NextParser + 'a,
    {
        BoxedParser::new(and_then(self, f))
    }
}

impl<'a, F, Output> Parser<'a, Output> for F
where
    F: Fn(&'a str) -> ParseResult<Output>,
{
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output> {
        self(input)
    }
}

/// 更新的 map 实现
fn map<'a, P, F, A, B>(parser: P, map_fn: F) -> impl Parser<'a, B>
where
    P: Parser<'a, A>,
    F: Fn(A) -> B,
{
    move |input| {
        parser
            .parse(input)
            .map(|(next_input, result)| (next_input, map_fn(result)))
    }
}

fn pair_v2<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, (R1, R2)>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    move |input| match parser1.parse(input) {
        Ok((next_input, result1)) => match parser2.parse(next_input) {
            Ok((final_input, result2)) => Ok((final_input, (result1, result2))),
            Err(err) => Err(err),
        },
        Err(err) => Err(err),
    }
}

fn pair_v3<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, (R1, R2)>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    move |input| {
        parser1.parse(input).and_then(|(next_input, result1)| {
            parser2
                .parse(next_input)
                .map(|(last_input, result2)| (last_input, (result1, result2)))
        })
    }
}

// fn pair<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, (R1, R2)>
// where
//     P1: Parser<'a, R1> + 'a,
//     P2: Parser<'a, R2> + 'a,
//     R1: 'a + Clone,
//     R2: 'a,
// {
//     parser1.and_then(move |result1| parser2.map(move |result2| (result1.clone(), result2)))
// }

fn pair<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, (R1, R2)>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    move |input| match parser1.parse(input) {
        Ok((next_input, result1)) => match parser2.parse(next_input) {
            Ok((final_input, result2)) => Ok((final_input, (result1, result2))),
            Err(err) => Err(err),
        },
        Err(err) => Err(err),
    }
}

fn left<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, R1>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    map(pair(parser1, parser2), |(left, _right)| left)
    // pair(parser1, parser2).map(|(left, _right)| left)
}

fn right<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, R2>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    map(pair(parser1, parser2), |(_left, right)| right)
    // pair(parser1, parser2).map(|(_left, right)| right)
}

/// 识别标识符
fn identifier(input: &str) -> ParseResult<String> {
    let mut matched = String::new();
    let mut chars = input.chars();
    match chars.next() {
        Some(next) if next.is_alphabetic() => matched.push(next),
        _ => return Err(input),
    }
    while let Some(next) = chars.next() {
        if next.is_alphanumeric() || next == '-' {
            matched.push(next);
        } else {
            break;
        }
    }
    let next_index = matched.len();
    Ok((&input[next_index..], matched))
}

fn one_or_more<'a, P, A>(parser: P) -> impl Parser<'a, Vec<A>>
where
    P: Parser<'a, A>,
{
    move |mut input| {
        let mut result = Vec::new();
        if let Ok((next_input, first_item)) = parser.parse(input) {
            input = next_input;
            result.push(first_item);
        } else {
            return Err(input);
        }
        while let Ok((next_input, next_item)) = parser.parse(input) {
            input = next_input;
            result.push(next_item);
        }
        Ok((input, result))
    }
}

fn zero_or_more<'a, P, A>(parser: P) -> impl Parser<'a, Vec<A>>
where
    P: Parser<'a, A>,
{
    move |mut input| {
        let mut result = Vec::new();
        while let Ok((next_input, next_item)) = parser.parse(input) {
            input = next_input;
            result.push(next_item);
        }

        Ok((input, result))
    }
}

// fn one_or_more<'a, P, A>(parser: P) -> impl Parser<'a, Vec<A>>
// where
//     P: Parser<'a, A>
// {
//     map(pair(parser, zero_or_more(parser)), |(head, mut tail)| {
//         tail.insert(0, head);
//         tail
//     })
// }

fn any_char(input: &str) -> ParseResult<char> {
    match input.chars().next() {
        Some(next) => Ok((&input[next.len_utf8()..], next)),
        _ => Err(input),
    }
}

fn pred<'a, P, A, F>(parser: P, predicate: F) -> impl Parser<'a, A>
where
    P: Parser<'a, A>,
    F: Fn(&A) -> bool,
{
    move |input| {
        if let Ok((next_input, value)) = parser.parse(input) {
            if predicate(&value) {
                return Ok((next_input, value));
            }
        }
        Err(input)
    }
}

fn whitespace_char<'a>() -> impl Parser<'a, char> {
    pred(any_char, |c| c.is_whitespace())
}

/// 解析 1 个或多个空格符
fn space1<'a>() -> impl Parser<'a, Vec<char>> {
    one_or_more(whitespace_char())
}

/// 解析 0 个或多个空格符
fn space0<'a>() -> impl Parser<'a, Vec<char>> {
    zero_or_more(whitespace_char())
}

/// 解析引号内的值
fn quoted_string_v1<'a>() -> impl Parser<'a, String> {
    map(
        right(
            match_literal("\""),
            left(
                zero_or_more(pred(any_char, |c| *c != '"')),
                match_literal("\""),
            ),
        ),
        |chars| chars.into_iter().collect(),
    )
}

fn attribute_pair<'a>() -> impl Parser<'a, (String, String)> {
    pair(identifier, right(match_literal("="), quoted_string()))
}

/// 解析属性
fn attributes<'a>() -> impl Parser<'a, Vec<(String, String)>> {
    zero_or_more(right(space1(), attribute_pair()))
}

fn element_start<'a>() -> impl Parser<'a, (String, Vec<(String, String)>)> {
    right(match_literal("<"), pair(identifier, attributes()))
}

// fn single_element_v1<'a>() -> impl Parser<'a, Element> {
//     map(
//         left(element_start(), match_literal("/>")),
//         |(name, attributes)| Element {
//             name,
//             attributes,
//             children: vec![],
//         },
//     )
// }

struct BoxedParser<'a, Output> {
    parser: Box<dyn Parser<'a, Output> + 'a>,
}

impl<'a, Output> BoxedParser<'a, Output> {
    fn new<P>(parser: P) -> Self
    where
        P: Parser<'a, Output> + 'a,
    {
        BoxedParser {
            parser: Box::new(parser),
        }
    }
}

impl<'a, Output> Parser<'a, Output> for BoxedParser<'a, Output> {
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output> {
        self.parser.parse(input)
    }
}

fn quoted_string<'a>() -> impl Parser<'a, String> {
    right(
        match_literal("\""),
        left(
            // zero_or_more(pred(any_char, |c| *c != '"')),
            zero_or_more(any_char.pred(|c| *c != '"')),
            match_literal("\""),
        ),
    )
    .map(|chars| chars.into_iter().collect())
}

fn single_element<'a>() -> impl Parser<'a, Element> {
    left(element_start(), match_literal("/>")).map(|(name, attributes)| Element {
        name,
        attributes,
        children: vec![],
    })
}

fn open_element<'a>() -> impl Parser<'a, Element> {
    left(element_start(), match_literal(">")).map(|(name, attributes)| Element {
        name,
        attributes,
        children: vec![],
    })
}

// 先尝试解析器1，再尝试解析器2
fn either<'a, P1, P2, A>(parser1: P1, parser2: P2) -> impl Parser<'a, A>
where
    P1: Parser<'a, A>,
    P2: Parser<'a, A>,
{
    move |input| match parser1.parse(input) {
        ok @ Ok(_) => ok,
        Err(_) => parser2.parse(input),
    }
}

fn element_v1<'a>() -> impl Parser<'a, Element> {
    either(single_element(), open_element())
}

fn close_element<'a>(expected_name: String) -> impl Parser<'a, String> {
    right(match_literal("</"), left(identifier, match_literal(">")))
        .pred(move |name| name == &expected_name)
}

// fn parent_element_v1<'a>() -> impl Parser<'a, Element> {
//     pair(
//         open_element(),
//         left(zero_or_more(element()), close_element()),
//     )
// }

fn and_then<'a, P, F, A, B, NextP>(parser: P, f: F) -> impl Parser<'a, B>
where
    P: Parser<'a, A>,
    NextP: Parser<'a, B>,
    F: Fn(A) -> NextP,
{
    move |input| match parser.parse(input) {
        Ok((next_input, result)) => f(result).parse(next_input),
        Err(err) => Err(err),
    }
}

fn parent_element<'a>() -> impl Parser<'a, Element> {
    open_element().and_then(|el| {
        left(zero_or_more(element()), close_element(el.name.clone())).map(move |children| {
            let mut el = el.clone();
            el.children = children;
            el
        })
    })
}

fn whitespace_wrap<'a, P, A>(parser: P) -> impl Parser<'a, A>
where
    P: Parser<'a, A>,
{
    right(space0(), left(parser, space0()))
}

fn element<'a>() -> impl Parser<'a, Element> {
    whitespace_wrap(either(single_element(), parent_element()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_letter_a() {
        let input = "abcd";
        let res = the_letter_a(input);
        println!("{:?}", res);
        assert!(res == Ok(("bcd", ())));
    }

    #[test]
    fn test_match_literal() {
        let parse_func = match_literal("hello world");
        assert_eq!(Ok(("", ())), parse_func("hello world"));
        assert_eq!(Ok((" hi ", ())), parse_func("hello world hi "));
        assert_eq!(Err("hello Rust"), parse_func("hello Rust"));
    }

    #[test]
    fn test_identifier_v1() {
        assert_eq!(
            Ok(("", "hello-world".to_string())),
            identifier("hello-world")
        );
        assert_eq!(
            Ok((" a identifier", "not".to_string())),
            identifier("not a identifier")
        );
    }

    #[test]
    fn test_pair() {
        let tag_opener = right(match_literal("<"), identifier);
        assert_eq!(
            tag_opener.parse("<my-first-element/>"),
            Ok(("/>", "my-first-element".to_string()))
        );
    }

    #[test]
    fn test_identifier() {
        let tag_opener = right(match_literal("<"), identifier);
        assert_eq!(
            tag_opener.parse("<my-first-element/>"),
            Ok(("/>", "my-first-element".to_string()))
        );
        assert_eq!(tag_opener.parse("test-parse"), Err("test-parse"));
        assert_eq!(tag_opener.parse("!hello"), Err("!hello"));
    }

    #[test]
    fn test_one_or_more() {
        // 解析必须以 ha 开头的字符
        let parser = one_or_more(match_literal("ha"));
        assert_eq!(parser.parse("hahaha"), Ok(("", vec![(), (), ()])));
        assert_eq!(parser.parse("ha123"), Ok(("123", vec![()])));
        assert_eq!(parser.parse("aha123"), Err("aha123"));
    }

    #[test]
    fn test_zero_or_more() {
        // 解析必须以 ha 开头的字符
        let parser = zero_or_more(match_literal("ha"));
        assert_eq!(parser.parse("hahaha"), Ok(("", vec![(), (), ()])));
        // 匹配到 0 次
        assert_eq!(parser.parse("ahaaha"), Ok(("ahaaha", vec![])));
    }

    #[test]
    fn test_pred() {
        let parser = pred(any_char, |c| *c == 'o');
        assert_eq!(Ok(("mg", 'o')), parser.parse("omg"));
        assert_eq!(Err("lol"), parser.parse("lol"));
    }

    #[test]
    fn test_quoted_string() {
        assert_eq!(
            quoted_string().parse("\"Hello world\""),
            Ok(("", "Hello world".to_string()))
        );
    }

    #[test]
    fn attribute_parser() {
        assert_eq!(
            Ok((
                "",
                vec![
                    ("one".to_string(), "1".to_string()),
                    ("two".to_string(), "2".to_string())
                ]
            )),
            attributes().parse(" one=\"1\" two=\"2\"")
        );
    }

    #[test]
    fn test_single_element() {
        assert_eq!(
            Ok((
                "",
                Element {
                    name: "div".to_string(),
                    attributes: vec![("class".to_string(), "float".to_string())],
                    children: vec![]
                }
            )),
            single_element().parse("<div class=\"float\">")
        );
    }

    #[test]
    fn xml_parser() {
        let doc = r#"
            <top label="Top">
                <semi-bottom label="Bottom"/>
                <middle>
                    <bottom label="Another bottom"/>
                </middle>
            </top>"#;
        let parsed_doc = Element {
            name: "top".to_string(),
            attributes: vec![("label".to_string(), "Top".to_string())],
            children: vec![
                Element {
                    name: "semi-bottom".to_string(),
                    attributes: vec![("label".to_string(), "Bottom".to_string())],
                    children: vec![],
                },
                Element {
                    name: "middle".to_string(),
                    attributes: vec![],
                    children: vec![Element {
                        name: "bottom".to_string(),
                        attributes: vec![("label".to_string(), "Another bottom".to_string())],
                        children: vec![],
                    }],
                },
            ],
        };
        assert_eq!(Ok(("", parsed_doc)), element().parse(doc));
    }
}
