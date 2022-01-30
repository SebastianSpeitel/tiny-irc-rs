use crate::message::{Message, ParsedMessage};
use std::collections::VecDeque;
use std::mem::replace;

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
}

pub struct Parser {
    raw: String,
    pos: u16,
    prefix: Option<(u16, u16)>,
    nick: Option<(u16, u16)>,
    user: Option<(u16, u16)>,
    host: Option<(u16, u16)>,
    command: (u16, u16),
    params: Vec<(u16, u16)>,

    state: State,

    parsed: VecDeque<ParsedMessage>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            raw: "".to_string(),
            pos: 0,
            prefix: None,
            nick: None,
            user: None,
            host: None,
            command: (0, 0),
            params: Vec::new(),
            state: State::Start,
            parsed: VecDeque::new(),
        }
    }

    fn emit(&mut self) {
        self.pos = 0;
        self.raw.clear();
        return ();
        println!("emit");
        println!("{:?}", self.pos);
        let rest = self.raw.split_off(self.pos as usize);
        let raw = replace(&mut self.raw, rest);
        let prefix = self.prefix.take();
        let nick = self.nick.take();
        let user = self.user.take();
        let host = self.host.take();
        let command = replace(&mut self.command, (0, 0));
        let params = replace(&mut self.params, Vec::new());

        // self.parsed.push_back(ParsedMessage::new(
        //     raw, prefix, nick, user, host, command, params,
        // ));
        self.pos = 0;
    }

    #[inline(always)]
    fn parse_start(&mut self, iter: &mut impl Iterator<Item = u8>) {
        if let Some(c) = iter.next() {
            if c == b':' {
                self.pos = 0;
                self.state = State::PrefixNick { begin: 1 };
            } else if c == b'\r' || c == b'\n' {
            } else {
                self.pos = 0;
                self.state = State::Command { begin: 0 };
            }
        }
    }

    #[inline(always)]
    fn parse_prefix_nick(&mut self, iter: &mut impl Iterator<Item = u8>, begin: u16) {
        while let Some(c) = iter.next() {
            self.pos += 1;
            if c == b'!' {
                self.nick.replace((begin, self.pos));
                self.state = State::PrefixUser {
                    begin: self.pos + 1,
                    begin_prefix: begin,
                };
                break;
            } else if c == b'@' {
                self.nick.replace((begin, self.pos));
                self.state = State::PrefixHost {
                    begin: self.pos + 1,
                    begin_prefix: begin,
                };
                break;
            } else if c == b' ' {
                self.nick.replace((begin, self.pos));
                self.prefix.replace((begin, self.pos));
                self.state = State::Command {
                    begin: self.pos + 1,
                };
                break;
            }
        }
    }

    #[inline(always)]
    fn parse_prefix_user(
        &mut self,
        iter: &mut impl Iterator<Item = u8>,
        begin: u16,
        begin_prefix: u16,
    ) {
        while let Some(c) = iter.next() {
            self.pos += 1;
            if c == b'@' {
                self.user.replace((begin, self.pos));
                self.state = State::PrefixHost {
                    begin: self.pos + 1,
                    begin_prefix,
                };
                break;
            } else if c == b' ' {
                self.user.replace((begin, self.pos));
                self.prefix.replace((begin_prefix, self.pos));
                self.state = State::Command {
                    begin: self.pos + 1,
                };
                break;
            }
        }
    }

    #[inline(always)]
    fn parse_prefix_host(
        &mut self,
        iter: &mut impl Iterator<Item = u8>,
        begin: u16,
        begin_prefix: u16,
    ) {
        while let Some(c) = iter.next() {
            self.pos += 1;
            if c == b' ' {
                self.host.replace((begin, self.pos));
                self.prefix.replace((begin_prefix, self.pos));
                self.state = State::Command {
                    begin: self.pos + 1,
                };
                break;
            }
        }
    }

    #[inline(always)]
    fn parse_command(&mut self, iter: &mut impl Iterator<Item = u8>, begin: u16) {
        while let Some(c) = iter.next() {
            self.pos += 1;
            if c == b' ' {
                self.command = (begin, self.pos);
                self.state = State::Params;
                break;
            }
        }
    }

    #[inline(always)]
    fn parse_params(&mut self, iter: &mut impl Iterator<Item = u8>) {
        if let Some(c) = iter.next() {
            self.pos += 1;
            if c == b':' {
                self.state = State::ParamsTrailing {
                    begin: self.pos + 1,
                }
            } else if c == b'\r' {
                self.emit();
                self.state = State::Start
            } else {
                self.state = State::ParamsMiddle { begin: self.pos }
            }
        }
    }

    #[inline(always)]
    fn parse_params_middle(&mut self, iter: &mut impl Iterator<Item = u8>, begin: u16) {
        while let Some(c) = iter.next() {
            self.pos += 1;
            if c == b' ' {
                self.params.push((begin, self.pos));
                self.state = State::Params;
                break;
            } else if c == b'\r' {
                self.params.push((begin, self.pos));
                self.state = State::Start;
                break;
            }
        }
    }

    #[inline(always)]
    fn parse_params_trailing(&mut self, iter: &mut impl Iterator<Item = u8>, begin: u16) {
        while let Some(c) = iter.next() {
            self.pos += 1;
            if c == b'\r' {
                self.params.push((begin, self.pos));
                self.emit();
                self.state = State::Start;
                break;
            }
        }
    }

    pub fn parse(&mut self, msg: String) -> Result<(), String> {
        self.raw += &msg;
        let mut iter = msg.bytes();
        let mut max_iter = 100;
        while iter.len() > 0 && max_iter > 0 {
            max_iter -= 1;
            // println!("{:?}", self.state);
            match self.state {
                State::Start => self.parse_start(&mut iter),
                State::PrefixNick { begin } => self.parse_prefix_nick(&mut iter, begin),
                State::PrefixUser {
                    begin,
                    begin_prefix,
                } => {
                    self.parse_prefix_user(&mut iter, begin, begin_prefix);
                }
                State::PrefixHost {
                    begin,
                    begin_prefix,
                } => {
                    self.parse_prefix_host(&mut iter, begin, begin_prefix);
                }
                State::Command { begin } => self.parse_command(&mut iter, begin),
                State::Params => {
                    self.parse_params(&mut iter);
                }
                State::ParamsMiddle { begin } => {
                    self.parse_params_middle(&mut iter, begin);
                }
                State::ParamsTrailing { begin } => {
                    self.parse_params_trailing(&mut iter, begin);
                }
            }
        }
        Ok(())
    }
}

impl Iterator for Parser {
    type Item = ParsedMessage;

    fn next(&mut self) -> Option<Self::Item> {
        self.parsed.pop_front()
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
        parser.parse(msg).unwrap();
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
        parser.parse(msg).unwrap();
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

    #[bench]
    fn bench_parse_usual(b: &mut test::Bencher) {
        let msg =
            ":irc.example.com 001 test :Welcome to the Internet Relay Network\r\n".to_string();
        let mut parser = Parser::new();
        b.iter(|| parser.parse(msg.clone()));
    }

    #[bench]
    fn bench_parse_small(b: &mut test::Bencher) {
        let msg = "PING \r\n".to_string();
        let mut parser = Parser::new();
        b.iter(|| parser.parse(msg.clone()));
    }

    #[bench]
    fn bench_parse_long_nick(b: &mut test::Bencher) {
        let front = ":".to_string();
        let nick = "_".repeat(512 - 8);
        let back = " PING".to_string();
        let msg = format!("{}{}{}\r\n", front, nick, back);

        assert_eq!(msg.len(), 512);

        let mut parser = Parser::new();
        b.iter(|| parser.parse(msg.clone()));
    }

    #[bench]
    fn bench_parse_long_trailing(b: &mut test::Bencher) {
        let front = ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
        let back = "_".repeat(446);
        let msg = format!("{}{}\r\n", front, back);

        assert_eq!(msg.len(), 512);

        let mut parser = Parser::new();
        b.iter(|| parser.parse(msg.clone()));
    }
}
