extern crate bytes;
extern crate tokio;

use bytes::{BufMut, BytesMut};
use tokio::net::TcpStream;
use tokio::prelude::*;
use std::error::Error;

async fn connect() -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:6379").await?;
    println!("{:?}",stream);
    Ok(())
}

mod tests {
    use super::*;

    #[test]
    fn test_connect() {
        let res = connect();
        println!("{:?}", 123123);
        assert!(false);
    }
}


