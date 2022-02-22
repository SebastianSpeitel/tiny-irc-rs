use smallvec::SmallVec;

use crate::message::{prelude::*, ParsedMessage};
// use std::cell::{Cell, RefCell, RefMut};
// use std::collections::VecDeque;
use std::mem::{replace, take};

#[derive(Debug)]
enum State {
    Start,
    PrefixNick { begin: u16 },
    PrefixUser { begin: u16, begin_prefix: u16 },
    PrefixHost { begin: u16, begin_prefix: u16 },
    Command { begin: u16 },
    Params,
    ParamsMiddle { begin: u16 },
    ParamsTrailing { begin: u16 },
    End,
    Stop,
}

pub struct Parser {
    buffer: Vec<u8>,
}

struct BufferIter<T>
where
    T: Iterator,
{
    inner: T,
    pos: u16,
}

impl<T> BufferIter<T>
where
    T: Iterator,
{
    #[inline(always)]
    fn new(inner: T) -> Self {
        BufferIter { inner, pos: 0 }
    }
}

impl<T> Iterator for BufferIter<T>
where
    T: Iterator,
{
    type Item = (T::Item, u16);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|c| {
            self.pos += 1;
            (c, self.pos - 1)
        })
    }
}

impl<T> ExactSizeIterator for BufferIter<T>
where
    T: ExactSizeIterator,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl Parser {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    // fn emit(&mut self) {
    //     self.pos = 0;
    //     self.raw.clear();
    //     return ();
    //     println!("emit");
    //     println!("{:?}", self.pos);
    //     let rest = self.raw.split_off(self.pos as usize);
    //     let raw = replace(&mut self.raw, rest);
    //     let prefix = self.prefix.take();
    //     let nick = self.nick.take();
    //     let user = self.user.take();
    //     let host = self.host.take();
    //     let command = replace(&mut self.command, (0, 0));
    //     let params = replace(&mut self.params, Vec::new());

    //     // self.parsed.push_back(ParsedMessage::new(
    //     //     raw, prefix, nick, user, host, command, params,
    //     // ));
    //     self.pos = 0;
    // }

    // #[inline(always)]
    // fn build(&mut self, end: u16) -> ParsedMessage {
    //     let rest = self.buffer.split_off(end as usize);
    //     let raw_vec = replace(&mut self.buffer, rest).iter().cloned().collect();
    //     let raw = unsafe { String::from_utf8_unchecked(raw_vec) };
    //     let params = replace(&mut self.params, Vec::new());
    //     ParsedMessage::new(
    //         raw,
    //         self.prefix.take(),
    //         self.nick.take(),
    //         self.user.take(),
    //         self.host.take(),
    //         self.command,
    //         params, // self.params.drain(..).collect(),
    //     )
    // }

    #[inline(always)]
    fn parse_start(&self, iter: &mut impl Iterator<Item = (u8, u16)>) -> State {
        if let Some((c, pos)) = iter.next() {
            debug_assert_eq!(pos, 0);
            if c == b':' {
                return State::PrefixNick { begin: 1 };
            } else {
                return State::Command { begin: 0 };
            }
        }
        State::Stop
    }

    #[inline(always)]
    fn parse_prefix_nick(
        &self,
        iter: &mut impl Iterator<Item = (u8, u16)>,
        begin: u16,
        prefix: &mut Option<(u16, u16)>,
        nick: &mut Option<(u16, u16)>,
    ) -> State {
        while let Some((c, pos)) = iter.next() {
            if c == b'!' {
                nick.replace((begin, pos));
                return State::PrefixUser {
                    begin: pos + 1,
                    begin_prefix: begin,
                };
            } else if c == b'@' {
                nick.replace((begin, pos));
                return State::PrefixHost {
                    begin: pos + 1,
                    begin_prefix: begin,
                };
            } else if c == b' ' {
                nick.replace((begin, pos));
                prefix.replace((begin, pos));
                return State::Command { begin: pos + 1 };
            }
        }
        State::Stop
    }

    #[inline(always)]
    fn parse_prefix_user(
        &self,
        iter: &mut impl Iterator<Item = (u8, u16)>,
        begin: u16,
        begin_prefix: u16,
        prefix: &mut Option<(u16, u16)>,
        user: &mut Option<(u16, u16)>,
    ) -> State {
        while let Some((c, pos)) = iter.next() {
            if c == b'@' {
                user.replace((begin, pos));
                return State::PrefixHost {
                    begin: pos + 1,
                    begin_prefix,
                };
            } else if c == b' ' {
                user.replace((begin, pos));
                prefix.replace((begin_prefix, pos));
                return State::Command { begin: pos + 1 };
            }
        }
        State::Stop
    }

    #[inline(always)]
    fn parse_prefix_host(
        &self,
        iter: &mut impl Iterator<Item = (u8, u16)>,
        begin: u16,
        begin_prefix: u16,
        prefix: &mut Option<(u16, u16)>,
        host: &mut Option<(u16, u16)>,
    ) -> State {
        while let Some((c, pos)) = iter.next() {
            if c == b' ' {
                host.replace((begin, pos));
                prefix.replace((begin_prefix, pos));
                return State::Command { begin: pos + 1 };
            }
        }
        State::Stop
    }

    #[inline(always)]
    fn parse_command(
        &self,
        iter: &mut impl Iterator<Item = (u8, u16)>,
        begin: u16,
        command: &mut Option<(u16, u16)>,
    ) -> State {
        while let Some((c, pos)) = iter.next() {
            if c == b' ' {
                command.replace((begin, pos));
                return State::Params;
            }
            debug_assert_ne!(c as char, '\r');
            debug_assert_ne!(c as char, '\n');
        }
        State::Stop
    }

    #[inline(always)]
    fn parse_params(&self, iter: &mut impl Iterator<Item = (u8, u16)>) -> State {
        if let Some((c, pos)) = iter.next() {
            if c == b':' {
                return State::ParamsTrailing { begin: pos + 1 };
            } else if c == b'\r' {
                return State::End;
            } else {
                return State::ParamsMiddle { begin: pos };
            }
        }
        State::Stop
    }

    #[inline(always)]
    fn parse_params_middle(
        &self,
        iter: &mut impl Iterator<Item = (u8, u16)>,
        begin: u16,
        params: &mut SmallVec<[(u16, u16); 2]>,
    ) -> State {
        while let Some((c, pos)) = iter.next() {
            if c == b' ' {
                params.push((begin, pos));
                return State::Params;
            } else if c == b'\r' {
                params.push((begin, pos));
                return State::End;
            }
        }
        State::Stop
    }

    #[inline(always)]
    fn parse_params_trailing(
        &self,
        iter: &mut impl Iterator<Item = (u8, u16)>,
        begin: u16,
        params: &mut SmallVec<[(u16, u16); 2]>,
    ) -> State {
        while let Some((c, pos)) = iter.next() {
            if c == b'\r' {
                params.push((begin, pos));
                return State::End;
            }
        }
        State::Stop
    }

    pub fn push(&mut self, buf_in: String) {
        if self.buffer.is_empty() {
            self.buffer = buf_in.into_bytes();
        } else {
            self.buffer.append(&mut buf_in.into_bytes());
        }
    }

    pub fn push_buf(&mut self, buf_in: &[u8]) {
        if self.buffer.is_empty() {
            self.buffer = buf_in.to_vec();
        } else {
            self.buffer.append(&mut buf_in.to_vec());
        }
    }
}

impl Iterator for Parser {
    type Item = ParsedMessage;

    fn next(&mut self) -> Option<ParsedMessage> {
        let mut state = State::Start;
        // let buffer = &self.buffer;
        // let mut iter = BufferIter::new(buffer.iter().cloned());
        let mut pos: u16 = 0;
        let mut iter = self.buffer.iter().map(|c| {
            pos += 1;
            (*c, pos - 1)
        });

        // println!("pos: {:?}, len: {:?}", iter.pos, iter.len());
        // while let Some((c, pos)) = iter.next() {
        //     println!("{:?}", (c as char, pos));
        // }
        // println!("pos: {:?}, len: {:?}", iter.pos, iter.len());

        // return None;

        let mut prefix: Option<(u16, u16)> = None;
        let mut nick: Option<(u16, u16)> = None;
        let mut user: Option<(u16, u16)> = None;
        let mut host: Option<(u16, u16)> = None;
        let mut command: Option<(u16, u16)> = None;
        let mut params: SmallVec<[(u16, u16); 2]> = SmallVec::new();

        // println!("Start: {:?}", self.state);
        debug_assert!(matches!(state, State::Start));
        state = self.parse_start(&mut iter);
        // match self.state {
        //     State::Start => self.parse_start(&mut iter),
        //     _ => {}
        // }

        // println!("Nick?: {:?}", self.state);
        match state {
            State::PrefixNick { begin } => {
                state = self.parse_prefix_nick(&mut iter, begin, &mut prefix, &mut nick);
                //TODO: move PrefixUser and PrefixHost in here
            }
            State::Stop => return None,
            _ => {}
        }

        // println!("User?: {:?}", state);
        match state {
            State::PrefixUser {
                begin,
                begin_prefix,
            } => {
                state =
                    self.parse_prefix_user(&mut iter, begin, begin_prefix, &mut prefix, &mut user)
            }
            State::Stop => return None,
            _ => {}
        }

        // println!("Host?: {:?}", self.state);
        match state {
            State::PrefixHost {
                begin,
                begin_prefix,
            } => {
                state =
                    self.parse_prefix_host(&mut iter, begin, begin_prefix, &mut prefix, &mut host)
            }
            State::Stop => return None,
            _ => {}
        }

        // debug_assert!(matches!(state, State::Command { begin }));
        // println!("Command?: {:?}", state);
        match state {
            State::Command { begin } => state = self.parse_command(&mut iter, begin, &mut command),
            State::Stop => return None,
            _ => {}
        }

        // println!("Params?: {:?}", self.state);
        loop {
            // println!("End?: {:?}", self.state);
            match state {
                State::End => {
                    if let Some((c, pos)) = iter.next() {
                        if c == b'\n' {
                            let raw_vec = if pos as usize == self.buffer.len() - 1 {
                                take(&mut self.buffer)
                            } else {
                                let rest = self.buffer.split_off(pos as usize + 1);
                                replace(&mut self.buffer, rest)
                            };

                            let raw = unsafe { String::from_utf8_unchecked(raw_vec) };
                            return Some(ParsedMessage::new(
                                raw,
                                prefix,
                                nick,
                                user,
                                host,
                                command.unwrap(),
                                params,
                            ));
                        }
                    }
                    return None;
                }
                State::Stop => return None,
                _ => {}
            }

            // debug_assert!(matches!(state, State::Params));
            state = self.parse_params(&mut iter);
            // match self.state {
            //     State::Params => self.parse_params(&mut iter),
            //     _ => {}
            // }

            // println!("Params2?: {:?}", self.state);
            match state {
                State::ParamsMiddle { begin } => {
                    state = self.parse_params_middle(&mut iter, begin, &mut params);
                }
                State::ParamsTrailing { begin } => {
                    state = self.parse_params_trailing(&mut iter, begin, &mut params);
                }
                State::Stop => return None,
                _ => {}
            }
        }

        // while iter.len() > 0 {
        //     // println!("{:?}", self.state);
        //     match self.state {
        //         State::Start => self.parse_start(&mut iter),
        //         State::PrefixNick { begin } => self.parse_prefix_nick(&mut iter, begin),
        //         State::PrefixUser {
        //             begin,
        //             begin_prefix,
        //         } => {
        //             self.parse_prefix_user(&mut iter, begin, begin_prefix);
        //         }
        //         State::PrefixHost {
        //             begin,
        //             begin_prefix,
        //         } => {
        //             self.parse_prefix_host(&mut iter, begin, begin_prefix);
        //         }
        //         State::Command { begin } => self.parse_command(&mut iter, begin),
        //         State::Params => {
        //             self.parse_params(&mut iter);
        //         }
        //         State::ParamsMiddle { begin } => {
        //             self.parse_params_middle(&mut iter, begin);
        //         }
        //         State::ParamsTrailing { begin } => {
        //             self.parse_params_trailing(&mut iter, begin);
        //         }
        //         State::End => {
        //             if let Some((c, pos)) = iter.next() {
        //                 if c == b'\n' {
        //                     self.state = State::Start;
        //                     return Some(self.build(pos + 1));
        //                 }
        //             }
        //         }
        //     }
        // }
        // None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate test;

    #[test]
    fn test_parse() {
        let msg =
            ":irc.example.com 001 test :Welcome to the Internet Relay Network\r\n".to_string();
        let mut parser = Parser::new();
        parser.push(msg);
        let msg = parser.next().unwrap();

        // assert_eq!(
        //     msg.raw,
        //     ":irc.example.com 001 test :Welcome to the Internet Relay Network"
        // );
        assert_eq!(msg.command(), "001");
        assert_eq!(msg.prefix(), Some("irc.example.com".to_string()));
        assert_eq!(
            msg.params(),
            vec!["test", "Welcome to the Internet Relay Network"]
        );
    }

    #[test]
    fn test_parse_with_prefix() {
        let msg =
            ":<nick>!<user>@<user>.tmi.twitch.tv PRIVMSG #<channel> :This is a sample message\r\n"
                .to_string();
        let mut parser = Parser::new();
        parser.push(msg);
        let msg = parser.next().unwrap();

        assert_eq!(msg.command(), "PRIVMSG");
        assert_eq!(
            msg.prefix(),
            Some("<nick>!<user>@<user>.tmi.twitch.tv".to_string())
        );
        assert_eq!(msg.nick(), Some("<nick>".to_string()));
        assert_eq!(msg.user(), Some("<user>".to_string()));
        assert_eq!(msg.host(), Some("<user>.tmi.twitch.tv".to_string()));
        assert_eq!(msg.params(), vec!["#<channel>", "This is a sample message"]);
    }

    #[test]
    fn test_parse_two_messages() {
        let msg1 = ":irc.example.com 001 test :Message1\r\n".to_string();
        let msg2 = ":irc.example.com 001 test :Message2\r\n".to_string();
        let mut parser = Parser::new();
        parser.push(msg1);
        parser.push(msg2);
        let msg1 = parser.next().unwrap();
        let msg2 = parser.next().unwrap();

        assert_eq!(msg1.command(), "001");
        assert_eq!(msg1.params(), vec!["test", "Message1"]);
        assert_eq!(msg2.command(), "001");
        assert_eq!(msg2.params(), vec!["test", "Message2"]);
    }

    #[test]
    fn test_parse_incomplete() {
        let msg = ":irc.example.com 001 ".to_string();
        let mut parser = Parser::new();
        parser.push(msg);
        assert_eq!(parser.next(), None);

        parser.push("\r\n".to_string());
        let msg = parser.next();
        assert_ne!(msg, None);
        let msg = msg.unwrap();
        assert_eq!(msg.command(), "001");
        // assert_eq!(msg.params().len(), 0);
    }

    #[bench]
    fn bench_parse_usual(b: &mut test::Bencher) {
        let msg =
            ":irc.example.com 001 test :Welcome to the Internet Relay Network\r\n".to_string();
        let mut parser = Parser::new();
        b.iter(|| {
            parser.push(msg.clone());
            parser.next()
        });
    }

    #[bench]
    fn bench_parse_small(b: &mut test::Bencher) {
        let msg = "PING \r\n".to_string();
        let mut parser = Parser::new();
        b.iter(|| {
            parser.push(msg.clone());
            parser.next()
        });
    }

    #[bench]
    fn bench_parse_long_nick(b: &mut test::Bencher) {
        let front = ":".to_string();
        let nick = "_".repeat(512 - 9);
        let back = " PING ".to_string();
        let msg = format!("{}{}{}\r\n", front, nick, back);

        assert_eq!(msg.len(), 512);

        let mut parser = Parser::new();
        b.iter(|| {
            parser.push(msg.clone());
            parser.next()
        });
    }

    #[bench]
    fn bench_parse_long_trailing(b: &mut test::Bencher) {
        let front = ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
        let back = "_".repeat(446);
        let msg = format!("{}{}\r\n", front, back);

        assert_eq!(msg.len(), 512);

        let mut parser = Parser::new();
        b.iter(|| {
            parser.push(msg.clone());
            parser.next()
        });
    }

    #[bench]
    fn bench_parse_sequential1(b: &mut test::Bencher) {
        let msg =
            ":irc.example.com 001 test :Welcome to the Internet Relay Network\r\n".to_string();

        let mut parser = Parser::new();
        b.iter(|| {
            parser.push(msg.clone());
            parser.next();
            parser.push(msg.clone());
            parser.next()
        });
    }
    #[bench]
    fn bench_parse_sequential2(b: &mut test::Bencher) {
        let msg =
            ":irc.example.com 001 test :Welcome to the Internet Relay Network\r\n".to_string();

        let mut parser = Parser::new();
        b.iter(|| {
            parser.push(msg.clone());
            parser.push(msg.clone());
            parser.next();
            parser.next()
        });
    }
}
