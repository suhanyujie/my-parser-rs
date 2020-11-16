#![feature(unicode_internals)]

#[derive(Debug)]
struct Title {
    level: u8,
    title: String,
}

#[derive(Debug)]
struct TitleItem {
    title: Option<Title>,
    item_content: Vec<String>,
}

type ElementList = Vec<TitleItem>;

#[derive(Debug)]
struct MdBody {
    filename: String,
    content: ElementList,
}

/// 解析出一个字母 `a`
fn the_letter_a(input: &str) -> Result<(&str, ()), &str> {
    match input.chars().next() {
        Some('a') => Ok((&input['a'.len_utf8()..], ())),
        _ => Err(input),
    }
}

/// 解析一串字符串/中文
fn match_literal(expected: &'static str) -> impl Fn(&str) -> Result<(&str, ()), &str> {
    move |input| match input.get(0..expected.len()) {
        Some(next) if next == expected => Ok((&input[expected.len()..], ())),
        _ => Err(input),
    }
}

/// 类型声明
type ParseResult<'a, Output> = Result<(&'a str, Output), &'a str>;

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

/// 解析器对（一堆解析器)
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

trait ExtMatch {
    fn is_valid_string(self) -> bool;
}

impl ExtMatch for char {
    /// 是否是合法的字符串，包括中英文字符串
    fn is_valid_string(self) -> bool {
        match self {
            // '\u{4e00}'..='\u{9fa5}' => true, // 匹配中文
            '\n' => false,
            _ => true,
        }
    }
}

/// 识别标题项
fn recog_title(input: &str) -> Result<(&str, String), &str> {
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

/// map 实现
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

/// 识别标题内容
fn recog_title_content(input: &str) -> ParseResult<String> {
    let mut matched = String::new();
    let mut chars = input.chars();
    match chars.next() {
        Some(next) if next.is_valid_string() => matched.push(next),
        _ => return Err(input),
    }
    while let Some(next) = chars.next() {
        if next.is_valid_string() {
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

fn any_char(input: &str) -> ParseResult<char> {
    match input.chars().next() {
        Some(next) => Ok((&input[next.len_utf8()..], next)),
        _ => Err(input),
    }
}

fn whitespace_str<'a>() -> impl Parser<'a, char> {
    pred(any_char, |c| c.is_whitespace())
}

fn space0<'a>() -> impl Parser<'a, Vec<char>> {
    zero_or_more(whitespace_str())
}

fn space1<'a>() -> impl Parser<'a, Vec<char>> {
    one_or_more(whitespace_str())
}

fn title_content<'a>() -> impl Parser<'a, String> {
    map(
        right(
            match_literal(""),
            left(
                zero_or_more(pred(any_char, |c| *c != '\n')),
                match_literal("\n"),
            ),
        ),
        |chars| chars.into_iter().collect(),
    )
}

/// 解析标题等级
fn title_level(input: &str) -> ParseResult<String> {
    let mut level: u8 = 0;
    let mut level_str = String::new();
    let mut chars = input.chars();
    match chars.next() {
        Some('#') => {
            level += 1;
            level_str.push('#');
        }
        _ => {
            return Err(input);
        }
    }
    while let Some(c) = chars.next() {
        if c == '#' {
            level += 1;
            level_str.push('#');
        } else {
            break;
        }
    }
    let next_len = level_str.len();
    Ok((&input[next_len..], level_str))
}

fn title_level2(input: &str) -> ParseResult<u8> {
    let mut level: u8 = 0;
    let mut level_str = String::new();
    let mut chars = input.chars();
    match chars.next() {
        Some('#') => {
            level += 1;
            level_str.push('#');
        }
        _ => {
            // return Ok((&input[..], level));
            // level += 1;
            // level_str.push('#');
        }
    }
    while let Some(c) = chars.next() {
        if c == '#' {
            level += 1;
            level_str.push('#');
        } else {
            break;
        }
    }
    let next_len = level_str.len();
    Ok((&input[next_len..], level))
}

/// 解析标题等级以及标题内容
fn title_pair<'a>() -> impl Parser<'a, (u8, String)> {
    pair(title_level2, title_content())
}

mod tests {
    use super::*;

    #[test]
    fn test_struct() {
        let t1 = Title {
            level: 1,
            title: "title 1".to_string(),
        };
        assert_eq!(t1.title, "title 1".to_string());
    }

    #[test]
    fn test_is_valid_string() {
        let input = "this is 苏 \n - 内容1";
        let mut chars = input.chars();
        while let Some(next) = chars.next() {
            let res = if next.is_valid_string() { true } else { false };
            println!("[{}] -> {}", next, res);
            assert!(res);
        }
    }

    #[test]
    fn test_recog_title() {
        let parse_func = match_literal("#");
        assert_eq!(parse_func("# 你是"), Ok((" 你是", ())));
        assert_eq!(parse_func("## 你是"), Ok(("# 你是", ())));
        assert_eq!(parse_func("你是"), Err("你是"));
    }

    #[test]
    fn test_recog_title_content() {
        assert_eq!(
            Ok(("", "hello-title".to_string())),
            recog_title_content("hello-title")
        );
        assert_eq!(
            Ok(("", "一级标题".to_string())),
            recog_title_content("一级标题")
        );
        assert_eq!(
            recog_title_content("一级\n标题"),
            Ok(("\n标题", "一级".to_string()))
        );
    }

    #[test]
    fn test_pair() {
        let title_beginer = pair(match_literal("#"), recog_title_content);
        assert_eq!(
            title_beginer.parse("# this is a title\n"),
            Ok(("\n", ((), " this is a title".to_string())))
        );
    }

    #[test]
    fn test_right() {
        let righter = right(match_literal("#"), recog_title_content);
        assert_eq!(
            righter.parse("# title 1\n"),
            Ok(("\n", " title 1".to_string()))
        );
        assert_eq!(righter.parse("title 1"), Err("title 1"));
    }

    #[test]
    fn test_one_or_more() {
        let parser = one_or_more(match_literal("ha"));
        assert_eq!(parser.parse("hahaha"), Ok(("", vec![(), (), ()])));
        assert_eq!(parser.parse("no_title"), Err("no_title"));
    }

    #[test]
    fn test_zero_or_more() {
        let parser = zero_or_more(match_literal("ha"));
        assert_eq!(parser.parse("title"), Ok(("title", vec![])));
        assert_eq!(parser.parse("hatitle"), Ok(("title", vec![()])));
    }

    #[test]
    fn test_predicate() {
        let parser = pred(any_char, |c| *c == '#');
        assert_eq!(parser.parse("# title"), Ok((" title", '#')));
        assert_eq!(parser.parse("Begin content"), Err("Begin content"));
    }

    #[test]
    fn test_title_content() {
        assert_eq!(
            title_content().parse(" who is this\n"),
            Ok(("", " who is this".to_string()))
        );
    }

    #[test]
    fn test_title_pair() {
        assert_eq!(
            title_pair().parse("# why to do so? \n"),
            Ok(("", (1, " why to do so? ".to_string())))
        );
        assert_eq!(
            title_pair().parse("why to do so? \n"),
            Ok(("", (0, "why to do so? ".to_string())))
        );
    }
}
