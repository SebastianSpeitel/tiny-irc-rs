use std::{
    io::{BufRead, BufReader, Read},
    num::NonZeroUsize,
};
mod message;
pub use message::ParsedMessage;

mod steps {
    use core::hint::unreachable_unchecked;

    #[derive(Debug)]
    pub enum State {
        Nick,
        User,
        Host,
        Command,
        Params,
        ParamMiddle,
        ParamTrailing,
        End,

        EOF,
        Invalid,
    }

    #[inline(always)]
    pub fn start(buf: &[u8], offset: &mut usize) -> State {
        match buf.get(0) {
            Some(b':') => {
                *offset += 1;
                State::Nick
            }
            Some(_) => State::Command,
            None => State::EOF,
        }
    }

    #[inline(always)]
    pub fn nick(buf: &[u8], offset: &mut usize, nick: &mut Option<(u16, u16)>) -> State {
        use memchr::memchr3;
        match memchr3(b' ', b'!', b'@', buf) {
            Some(i) => {
                let end = *offset + i;
                nick.replace((*offset as u16, end as u16));
                *offset = end + 1;

                debug_assert!(matches!(buf[i], b' ' | b'!' | b'@'));
                match buf[i] {
                    b'!' => State::User,
                    b'@' => State::Host,
                    b' ' => State::Command,
                    _ => unsafe { unreachable_unchecked() },
                }
            }
            None => State::EOF,
        }
    }

    #[inline(always)]
    pub fn user(buf: &[u8], offset: &mut usize, user: &mut Option<(u16, u16)>) -> State {
        use memchr::memchr2;
        match memchr2(b' ', b'@', buf) {
            Some(i) => {
                let end = *offset + i;
                user.replace((*offset as u16, end as u16));
                *offset = end + 1;

                debug_assert!(matches!(buf[i], b' ' | b'@'));
                match buf[i] {
                    b'@' => State::Host,
                    b' ' => State::Command,
                    _ => unsafe { unreachable_unchecked() },
                }
            }
            None => State::EOF,
        }
    }

    #[inline(always)]
    pub fn host(buf: &[u8], offset: &mut usize, host: &mut Option<(u16, u16)>) -> State {
        use memchr::memchr;
        match memchr(b' ', buf) {
            Some(i) => {
                let end = *offset + i;
                host.replace((*offset as u16, end as u16));
                *offset = end + 1;
                State::Command
            }
            None => State::EOF,
        }
    }

    #[inline(always)]
    pub fn command(buf: &[u8], offset: &mut usize, command: &mut (u16, u16)) -> State {
        use memchr::memchr2;
        match memchr2(b' ', b'\r', buf) {
            Some(i) => {
                let end = *offset + i;
                *command = (*offset as u16, end as u16);

                debug_assert!(matches!(buf[i], b' ' | b'\r'));
                match buf[i] {
                    b' ' => {
                        *offset = end + 1;
                        State::Params
                    }
                    b'\r' => {
                        *offset = end;
                        State::End
                    }
                    _ => unsafe { unreachable_unchecked() },
                }
            }
            None => State::EOF,
        }
    }

    #[inline(always)]
    pub fn params(buf: &[u8], offset: &mut usize) -> State {
        match buf.get(0) {
            Some(b' ') => {
                *offset += 1;
                State::Params
            }
            Some(b':') => {
                *offset += 1;
                State::ParamTrailing
            }
            Some(_) => State::ParamMiddle,
            None => State::EOF,
        }
    }

    #[inline(always)]
    pub fn param_middle(
        buf: &[u8],
        offset: &mut usize,
        params: &mut smallvec::SmallVec<[(u16, u16); 2]>,
    ) -> State {
        use memchr::memchr2;
        match memchr2(b' ', b'\r', buf) {
            Some(i) => {
                let end = *offset + i;
                params.push((*offset as u16, end as u16));

                debug_assert!(matches!(buf[i], b' ' | b'\r'));
                match buf[i] {
                    b' ' => {
                        *offset = end + 1;
                        State::Params
                    }
                    b'\r' => {
                        *offset = end;
                        State::End
                    }
                    _ => unsafe { unreachable_unchecked() },
                }
            }
            None => State::EOF,
        }
    }

    #[inline(always)]
    pub fn param_trailing(
        buf: &[u8],
        offset: &mut usize,
        params: &mut smallvec::SmallVec<[(u16, u16); 2]>,
    ) -> State {
        use memchr::memchr;
        match memchr(b'\r', buf) {
            Some(i) => {
                let end = *offset + i;
                params.push((*offset as u16, end as u16));
                *offset = end;
                State::End
            }
            None => State::EOF,
        }
    }
}

type ParseResult<R, E> = Result<Option<(R, NonZeroUsize)>, E>;

pub trait Parsable
where
    Self: Sized,
    Self::Error: Into<Box<dyn std::error::Error>>,
{
    type Error;
    fn parse(buf: &[u8]) -> ParseResult<Self, Self::Error>;
}

#[derive(Debug)]
pub enum ParseError {
    Invalid,
    EOF,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Invalid => write!(f, "Invalid Message"),
            ParseError::EOF => write!(f, "Incomplete Message"),
        }
    }
}

impl std::error::Error for ParseError {}

impl Parsable for ParsedMessage {
    type Error = ParseError;

    fn parse(buf: &[u8]) -> ParseResult<Self, Self::Error> {
        use steps::{State::*, *};

        let mut pos: usize = 0;

        let mut msg = Self::new();

        let mut state = start(buf, &mut pos);

        if let Nick = state {
            state = nick(&buf[pos..], &mut pos, &mut msg.nick);

            match state {
                User => {
                    state = user(&buf[pos..], &mut pos, &mut msg.user);

                    if let Host = state {
                        state = host(&buf[pos..], &mut pos, &mut msg.host);
                    }
                }
                Host => {
                    state = host(&buf[pos..], &mut pos, &mut msg.host);
                }
                _ => {}
            };
        };

        if let Command = state {
            state = command(&buf[pos..], &mut pos, &mut msg.command);
        };

        loop {
            if let End = state {
                // pos is the index of the last character before the newline
                match buf.get(pos + 1) {
                    Some(b'\n') => {}
                    Some(_) => {
                        return Err(ParseError::Invalid);
                    }
                    None => {
                        return Ok(None);
                    }
                }

                msg.raw =
                    unsafe { std::str::from_utf8_unchecked(&buf.get_unchecked(..pos)) }.to_owned();

                // message + '\r\n' = pos + 2
                let consumed = unsafe { NonZeroUsize::new_unchecked(pos + 2) };
                return Ok(Some((msg, consumed)));
            };

            while let Params = state {
                state = params(&buf[pos..], &mut pos);
            }

            match state {
                ParamMiddle => {
                    state = param_middle(&buf[pos..], &mut pos, &mut msg.params);
                }
                ParamTrailing => {
                    state = param_trailing(&buf[pos..], &mut pos, &mut msg.params);
                }
                _ => {}
            };

            match state {
                EOF => {
                    return Ok(None);
                }
                Invalid => {
                    return Err(ParseError::Invalid);
                }
                _ => {}
            };
        }
    }
}

pub struct Parser<R, M>
where
    R: Read,
    M: Parsable,
{
    reader: BufReader<R>,
    _marker: std::marker::PhantomData<M>,
}

impl<R, M> Parser<R, M>
where
    R: Read,
    M: Parsable,
{
    pub fn new(source: R) -> Self {
        Self {
            reader: BufReader::with_capacity(512, source),
            _marker: std::marker::PhantomData,
        }
    }

    fn parse(&mut self) -> std::result::Result<Option<M>, Box<dyn std::error::Error>> {
        let buf = self.reader.fill_buf()?;
        // M::parse(buf)
        //     .map(|r| {
        //         r.map(|(msg, consumed)| {
        //             self.reader.consume(consumed.get());
        //             msg
        //         })
        //     })
        //     .map_err(|e| e.into())
        match M::parse(buf) {
            Ok(Some((msg, consumed))) => {
                self.reader.consume(consumed.get());
                Ok(Some(msg))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e.into()),
        }
        // match M::parse(buf) {
        //     ParseResult::Parsed { result, consumed } => {
        //         self.reader.consume(consumed.get());
        //         Ok(Some(result))
        //     }
        //     ParseResult::None => Ok(None),
        //     ParseResult::Error(e) => Err(e.into()),
        // }
    }
}

impl<T, M> Iterator for Parser<T, M>
where
    T: Read,
    M: Parsable,
{
    type Item = M;

    fn next(&mut self) -> Option<Self::Item> {
        self.parse().ok().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::super::message::prelude::*;
    use super::*;
    extern crate test;

    #[test]
    fn test_parse() {
        let msg = ":irc.example.com 001 test :Welcome to the Internet Relay Network\r\n".as_bytes();
        let msg = <ParsedMessage as Parsable>::parse(msg);
        let msg = msg.unwrap().unwrap().0;

        // assert_eq!(
        //     msg.raw,
        //     ":irc.example.com 001 test :Welcome to the Internet Relay Network"
        // );
        assert_eq!(msg.command(), "001");
        // assert_eq!(msg.prefix(), Some("irc.example.com".to_string()));
        assert_eq!(
            msg.params(),
            vec!["test", "Welcome to the Internet Relay Network"]
        );
    }

    #[test]
    fn test_parse_two() {
        let buf = "PING\r\n".as_bytes().repeat(2);
        let msg = <ParsedMessage as Parsable>::parse(&buf);
        let (msg, pos) = msg.unwrap().unwrap();
        assert_ne!(pos.get(), 0);

        let msg2 = <ParsedMessage as Parsable>::parse(&buf[pos.get()..]);
        let msg2 = msg2.unwrap().unwrap().0;

        assert_eq!(msg.command(), msg2.command());
    }

    #[test]
    fn test_parse_newline_incomplete() {
        let buf = "PING\r".as_bytes();
        let msg = <ParsedMessage as Parsable>::parse(buf);
        assert!(matches!(msg, Ok(None)));
    }

    #[test]
    fn test_parse_newline_invalid() {
        let buf = "PING\rPONG".as_bytes();
        let msg = <ParsedMessage as Parsable>::parse(buf);
        assert!(matches!(msg, Err(ParseError::Invalid)));
    }

    #[test]
    fn test_param_middle() {
        let msg = "000 param\r\n".as_bytes();
        let msg = <ParsedMessage as Parsable>::parse(msg);
        let msg = msg.unwrap().unwrap().0;

        assert_eq!(msg.params(), vec!["param"]);
    }

    #[test]
    fn test_param_trailing() {
        let msg = "000 :param\r\n".as_bytes();
        let msg = <ParsedMessage as Parsable>::parse(msg);
        let msg = msg.unwrap().unwrap().0;

        assert_eq!(msg.params(), vec!["param"]);
    }

    #[test]
    fn test_full_prefix() {
        let msg = ":nick!user@host 000\r\n".as_bytes();
        let msg = <ParsedMessage as Parsable>::parse(msg);
        let msg = msg.unwrap().unwrap().0;

        // assert_eq!(msg.prefix(), Some("nick!user@host".to_string()));
        assert_eq!(msg.nick(), Some("nick".to_string()));
        assert_eq!(msg.user(), Some("user".to_string()));
        assert_eq!(msg.host(), Some("host".to_string()));
    }

    #[test]
    fn test_parse_with_prefix() {
        let msg =
            ":<nick>!<user>@<user>.tmi.twitch.tv PRIVMSG #<channel> :This is a sample message\r\n"
                .as_bytes();
        let msg = <ParsedMessage as Parsable>::parse(msg);
        let msg = msg.unwrap().unwrap().0;

        assert_eq!(msg.command(), "PRIVMSG");
        // assert_eq!(
        //     msg.prefix(),
        //     Some("<nick>!<user>@<user>.tmi.twitch.tv".to_string())
        // );
        assert_eq!(msg.nick(), Some("<nick>".to_string()));
        assert_eq!(msg.user(), Some("<user>".to_string()));
        assert_eq!(msg.host(), Some("<user>.tmi.twitch.tv".to_string()));
        assert_eq!(msg.params(), vec!["#<channel>", "This is a sample message"]);
    }

    #[test]
    fn test_parse_incomplete() {
        let msgs = vec![
            ":",
            ":nick",
            ":nick!",
            ":nick!user",
            ":nick!user@",
            ":nick!user@host",
            ":nick!user@host ",
            ":nick!user@host 001",
            ":nick!user@host 001 ",
            ":nick!user@host 001 param",
            ":nick!user@host 001 :",
            ":nick!user@host 001 :trailing",
        ];

        for msg in msgs {
            dbg!(msg);
            let msg = <ParsedMessage as Parsable>::parse(msg.as_bytes());
            assert!(msg.unwrap().is_none());
        }
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    extern crate test;
    use test::Bencher;

    #[bench]
    fn parse_usual(b: &mut Bencher) {
        let msg = ":irc.example.com 001 test :Welcome to the Internet Relay Network\r\n".as_bytes();

        assert!(matches!(
            <ParsedMessage as Parsable>::parse(msg),
            Ok(Some(_))
        ));
        b.iter(|| <ParsedMessage as Parsable>::parse(msg));
    }

    #[bench]
    fn parse_small(b: &mut test::Bencher) {
        let msg = "PING\r\n".as_bytes();

        assert!(matches!(
            <ParsedMessage as Parsable>::parse(msg),
            Ok(Some(_))
        ));
        b.iter(|| <ParsedMessage as Parsable>::parse(msg));
    }

    #[bench]
    fn parse_long_nick(b: &mut test::Bencher) {
        let front = ":".to_string();
        let nick = "_".repeat(512 - 9);
        let back = " PING ".to_string();
        let msg = format!("{}{}{}\r\n", front, nick, back);

        let msg = msg.as_bytes();

        assert_eq!(msg.len(), 512);

        assert!(matches!(
            <ParsedMessage as Parsable>::parse(msg),
            Ok(Some(_))
        ));
        b.iter(|| <ParsedMessage as Parsable>::parse(msg));
    }

    #[bench]
    fn parse_long_trailing(b: &mut test::Bencher) {
        let front = ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
        let back = "_".repeat(446);
        let msg = format!("{}{}\r\n", front, back);
        let msg = msg.as_bytes();

        assert_eq!(msg.len(), 512);

        assert!(matches!(
            <ParsedMessage as Parsable>::parse(msg),
            Ok(Some(_))
        ));
        b.iter(|| <ParsedMessage as Parsable>::parse(msg));
    }

    #[bench]
    fn parse_sequential1(b: &mut test::Bencher) {
        let msg = ":irc.example.com 001 test :Welcome to the Internet Relay Network\r\n".as_bytes();

        assert!(matches!(
            <ParsedMessage as Parsable>::parse(msg),
            Ok(Some(_))
        ));
        b.iter(|| {
            <ParsedMessage as Parsable>::parse(msg);
            <ParsedMessage as Parsable>::parse(msg)
        });
    }

    #[bench]
    fn parse_privmsg_10(b: &mut test::Bencher) {
        let msg = format!(":irc.example.com PRIVMSG #test :{}\r\n", "_".repeat(10));
        let msg = msg.as_bytes();

        assert!(matches!(
            <ParsedMessage as Parsable>::parse(msg),
            Ok(Some(_))
        ));
        b.iter(|| <ParsedMessage as Parsable>::parse(msg));
    }

    #[bench]
    fn parse_privmsg_20(b: &mut test::Bencher) {
        let msg = format!(":irc.example.com PRIVMSG #test :{}\r\n", "_".repeat(20));
        let msg = msg.as_bytes();

        assert!(matches!(
            <ParsedMessage as Parsable>::parse(msg),
            Ok(Some(_))
        ));
        b.iter(|| <ParsedMessage as Parsable>::parse(msg));
    }

    #[bench]
    fn parse_privmsg_30(b: &mut test::Bencher) {
        let msg = format!(":irc.example.com PRIVMSG #test :{}\r\n", "_".repeat(30));
        let msg = msg.as_bytes();

        assert!(matches!(
            <ParsedMessage as Parsable>::parse(msg),
            Ok(Some(_))
        ));
        b.iter(|| <ParsedMessage as Parsable>::parse(msg));
    }

    #[bench]
    fn parse_privmsg_40(b: &mut test::Bencher) {
        let msg = format!(":irc.example.com PRIVMSG #test :{}\r\n", "_".repeat(40));
        let msg = msg.as_bytes();

        assert!(matches!(
            <ParsedMessage as Parsable>::parse(msg),
            Ok(Some(_))
        ));
        b.iter(|| <ParsedMessage as Parsable>::parse(msg));
    }

    #[bench]
    fn parse_privmsg_50(b: &mut test::Bencher) {
        let msg = format!(":irc.example.com PRIVMSG #test :{}\r\n", "_".repeat(50));
        let msg = msg.as_bytes();

        assert!(matches!(
            <ParsedMessage as Parsable>::parse(msg),
            Ok(Some(_))
        ));
        b.iter(|| <ParsedMessage as Parsable>::parse(msg));
    }

    #[bench]
    fn parse_privmsg_100(b: &mut test::Bencher) {
        let msg = format!(":irc.example.com PRIVMSG #test :{}\r\n", "_".repeat(100));
        let msg = msg.as_bytes();

        assert!(matches!(
            <ParsedMessage as Parsable>::parse(msg),
            Ok(Some(_))
        ));
        b.iter(|| <ParsedMessage as Parsable>::parse(msg));
    }

    #[bench]
    fn parse_privmsg_200(b: &mut test::Bencher) {
        let msg = format!(":irc.example.com PRIVMSG #test :{}\r\n", "_".repeat(200));
        let msg = msg.as_bytes();

        assert!(matches!(
            <ParsedMessage as Parsable>::parse(msg),
            Ok(Some(_))
        ));
        b.iter(|| <ParsedMessage as Parsable>::parse(msg));
    }
}
