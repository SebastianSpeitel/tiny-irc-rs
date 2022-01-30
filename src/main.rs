// #![feature(async_stream)]
#![feature(test)]

use futures::{Stream};
use std::io::{Read, Result, Write};
use std::net::TcpStream;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread::sleep;
use std::time::Duration;
pub mod message;
mod parser;
// use message::{from, BaseMsg, Message, PRIVMSG};
use std::mem::{size_of};

struct Chat {}

impl Stream for Chat {
    type Item = String;
    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(Some("Hello".to_string()))
    }
    // fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    //     println!("poll_next");
    //     Poll::Ready(None)
    // }
}

// impl StreamExt for Con {
// }

// fn foo<'a>() -> BaseMsg {
//     let msg_str = ":test".to_string();
//     let msg = from(&msg_str);

//     println!("{:?}", msg);

//     msg
// }

#[tokio::main]
async fn main() -> Result<()> {
    println!("start");

    // let nop = Message::NOOP;

    println!("(u8, u8): {:?}", size_of::<(u8, u8)>());
    println!("Option<(u8, u8)>: {:?}", size_of::<Option<(u8, u8)>>());
    println!(
        "Option<Option<(u8, u8)>>: {:?}",
        size_of::<Option<Option<(u8, u8)>>>()
    );
    println!("{:?}", size_of::<(u16, u16)>());
    println!("{:?}", size_of::<String>());
    // println!("{:?}", size_of::<Option<(usize, usize)>>());
    // println!("{:?}", size_of::<Option<usize>>());
    // println!("{:?}", size_of::<Message>());
    // println!("{:?}", size_of_val(&Message::NOOP));
    // println!("{:?}", size_of::<PRIVMSG>());

    // println!("{:?}", nop);

    // let msg = foo();
    // println!("{:?}", msg.prefix());

    let mut stream = TcpStream::connect("irc.chat.twitch.tv:6667")?;
    let pass = String::from("foobar");
    stream.write(format!("PASS {}\r\n", pass).as_bytes())?;
    stream.write(format!("NICK {}\r\n", "justinfan4514").as_bytes())?;

    sleep(Duration::from_secs(1));

    let mut buf: [u8; 1024] = [0; 1024];
    stream.read(&mut buf)?;
    println!("{}", String::from_utf8_lossy(&buf));

    // let mut con = Chat {};

    // while let Some(msg) = con.next().await {
    //     println!("{msg}");
    // }
    Ok(())
}

// async fn sum_with_next(mut stream: Pin<&mut Chat>) -> i32 {
//     use futures::stream::StreamExt; // for `next`
//     let mut sum = 0;
//     while let Some(item) = stream.next().await {
//         sum += item;
//     }
//     sum
// }

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // pub fn test_foo() {
    //     let msg_str = ":test".to_string();
    //     let msg = from(&msg_str);

    //     println!("{:?}", msg);
    // }
}
