#[derive(Clone, Debug, PartialEq, Eq)]
struct Element {
    name: String,
    attributes: Vec<(String, String)>,
    children: Vec<Element>,
}

// 解析出一个字母 `a`
fn the_letter_a(input: &str) -> Result<(&str, ()), &str> {
    match input.chars().next() {
        Some('a') => Ok((&input['a'.len_utf8()..], ())),
        _ => Err(input),
    }
}

// 解析一串字符串
fn match_literal(expected: &'static str) -> impl Fn(&str) -> Result<(&str, ()), &str> {
    move |input| match input.get(0..expected.len()) {
        Some(next) if next == expected => Ok((&input[expected.len()..], ())),
        _ => Err(input),
    }
}

// 识别标识符
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
        let input = "hello-world";
        let res = identifier(input);
        assert_eq!(Ok(("", "hello-world".to_string())), res);
    }
}
