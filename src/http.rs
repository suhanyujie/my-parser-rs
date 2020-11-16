//! http 协议解析器
//! 参考 https://github.com/sozu-proxy/sozu/blob/master/lib/src/protocol/http/parser.rs


/// http 中的 method
#[derive(PartialEq,Debug,Clone)]
pub enum Method {
  Get,
  Post,
  Head,
  Options,
  Put,
  Delete,
  Trace,
  Connect,
  Custom(String),
}

impl Method {
    pub fn new(s: &[u8]) -> Method {
        if compare_no_case(&s, b"GET") {
            return Method::Get;
        } else if compare_no_case(&s, b"POST"){
            return Method::Post;
        }
        Method::Custom(String::from(unsafe{std::str::from_utf8_unchecked(s)}))
    }
}

pub fn compare_no_case(left: &[u8], right: &[u8]) ->bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter().zip(right).all(|(a, b)| match (*a, *b){
        (0...64, 0...64) | (91...96, 91...96) | (123...255, 123...255) => a == b,
        (65...90, 65...90) | (97...122, 97...122) | (65...90, 97...122) | (97...122, 65...90) => *a | 0b00_10_00_00 == *b | 0b00_10_00_00,
        _ => false,
    })
}

mod tests {
    use super::*;

    #[test]
    fn test_method_new() {
        let b1 = b"POST";
        assert_eq!(Method::Post, Method::new(b1));
    }
}
