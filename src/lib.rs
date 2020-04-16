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

/// 识别标识符
fn identifier(input: &str) -> Result<(&str, String), &str> {
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
fn pair<P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Fn(&str) -> Result<(&str, (R1, R2)), &str>
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
    fn test_identifier() {
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
        let tag_opener = pair(match_literal("<"), identifier);
        assert_eq!(
            tag_opener("<my-first-element/>"),
            Ok(("/>", ((), "my-first-element".to_string())))
        );
    }
}
