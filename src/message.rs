use std::fmt::{Debug, Display, Formatter, Result as FResult};

pub trait Message {
    fn command(&self) -> String;
    fn params(&self) -> Vec<String>;
    fn prefix(&self) -> Option<String>;
    fn nick(&self) -> Option<String>;
    fn user(&self) -> Option<String>;
    fn host(&self) -> Option<String>;
}

// #[derive(Debug, PartialEq)]
pub struct ParsedMessage {
    raw: String,
    command: (u16, u16),
    params: Vec<(u16, u16)>,
    prefix: Option<(u16, u16)>,
    nick: Option<(u16, u16)>,
    user: Option<(u16, u16)>,
    host: Option<(u16, u16)>,
}

impl PartialEq for ParsedMessage {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl Display for ParsedMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        write!(f, "{}", self.raw)
    }
}

impl Debug for ParsedMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        write!(f, "{}", self.raw)
    }
}

impl Message for ParsedMessage {
    fn command(&self) -> String {
        let (begin, end) = self.command;
        debug_assert!(begin <= end);
        // self.raw[begin as usize..end as usize].to_string()
        unsafe {
            self.raw
                .get_unchecked(begin as usize..end as usize)
                .to_string()
        }
    }
    fn params(&self) -> Vec<String> {
        self.params
            .iter()
            .map(|&(begin, end)| unsafe {
                self.raw
                    .get_unchecked(begin as usize..end as usize)
                    .to_string()
            })
            .collect()
    }
    fn prefix(&self) -> Option<String> {
        self.prefix.map(|(begin, end)| unsafe {
            self.raw
                .get_unchecked(begin as usize..end as usize)
                .to_string()
        })
    }
    fn nick(&self) -> Option<String> {
        self.nick.map(|(begin, end)| unsafe {
            self.raw
                .get_unchecked(begin as usize..end as usize)
                .to_string()
        })
    }
    fn user(&self) -> Option<String> {
        self.user.map(|(begin, end)| unsafe {
            self.raw
                .get_unchecked(begin as usize..end as usize)
                .to_string()
        })
    }
    fn host(&self) -> Option<String> {
        self.host.map(|(begin, end)| unsafe {
            self.raw
                .get_unchecked(begin as usize..end as usize)
                .to_string()
        })
    }
}

impl ParsedMessage {
    pub fn new(
        raw: String,
        prefix: Option<(u16, u16)>,
        nick: Option<(u16, u16)>,
        user: Option<(u16, u16)>,
        host: Option<(u16, u16)>,
        command: (u16, u16),
        params: Vec<(u16, u16)>,
    ) -> Self {
        Self {
            raw,
            command,
            params,
            prefix,
            nick,
            user,
            host,
        }
    }

    /// Parse a message from a string.
    /// Example:
    /// ```
    /// let msg = ParsedMessage::from(":irc.example.com 001 test :Welcome to the Internet Relay Network");
    /// ```
    pub fn parse(raw: String) -> Self {
        let mut prefix: Option<(u16, u16)> = None;
        let mut nick: Option<(u16, u16)> = None;
        let mut user: Option<(u16, u16)> = None;
        let mut host: Option<(u16, u16)> = None;
        let mut command: Option<(u16, u16)> = None;
        let mut params: Vec<(u16, u16)> = Vec::new();

        enum State {
            Initial,
            PrefixNick { begin: u16 },
            PrefixUser { begin: u16, begin_prefix: u16 },
            PrefixHost { begin: u16, begin_prefix: u16 },
            Command { begin: u16 },
            Params,
            ParamsMiddle { begin: u16 },
            ParamsTrailing { begin: u16 },
        }

        let mut state = State::Initial;

        for (i, b) in raw.bytes().enumerate() {
            // if b == b'\r' || b == b'\n' {
            //     break;
            // }
            match state {
                State::Initial => match b {
                    b':' => {
                        state = State::PrefixNick {
                            begin: i as u16 + 1,
                        }
                    }
                    _ => {
                        state = State::Command { begin: i as u16 };
                    }
                },
                State::PrefixNick { begin } => match b {
                    b'!' => {
                        nick = Some((begin, i as u16));

                        state = State::PrefixUser {
                            begin: i as u16 + 1,
                            begin_prefix: begin,
                        };
                    }
                    b'@' => {
                        nick = Some((begin, i as u16));

                        state = State::PrefixHost {
                            begin: i as u16 + 1,
                            begin_prefix: begin,
                        };
                    }
                    b' ' => {
                        nick = Some((begin, i as u16));
                        prefix = Some((begin, i as u16));

                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                    }
                    _ => {}
                },
                State::PrefixUser {
                    begin,
                    begin_prefix,
                } => match b {
                    b'@' => {
                        user = Some((begin, i as u16));

                        state = State::PrefixHost {
                            begin: i as u16 + 1,
                            begin_prefix,
                        };
                    }
                    b' ' => {
                        user = Some((begin, i as u16));
                        prefix = Some((begin_prefix, i as u16));

                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                    }
                    _ => {}
                },
                State::PrefixHost {
                    begin,
                    begin_prefix,
                } => match b {
                    b' ' => {
                        host = Some((begin, i as u16));
                        prefix = Some((begin_prefix, i as u16));

                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                    }
                    _ => {}
                },
                State::Command { begin } => match b {
                    b' ' => {
                        command = Some((begin, i as u16));

                        state = State::Params;
                    }
                    _ => {}
                },
                State::Params => match b {
                    b' ' => {}
                    b':' => {
                        state = State::ParamsTrailing {
                            begin: i as u16 + 1,
                        };
                        // break;
                    }
                    _ => {
                        state = State::ParamsMiddle { begin: i as u16 };
                    }
                },
                State::ParamsMiddle { begin } => match b {
                    b' ' => {
                        params.push((begin, i as u16));

                        state = State::Params;
                    }
                    _ => {}
                },
                State::ParamsTrailing { begin: _ } => match b {
                    _ => {}
                },
            }
        }

        match state {
            State::Command { begin } => {
                command = Some((begin, raw.len() as u16));
            }
            State::ParamsTrailing { begin } => {
                params.push((begin, raw.len() as u16));
            }
            State::ParamsMiddle { begin } => {
                params.push((begin, raw.len() as u16));
            }
            _ => {}
        }

        Self {
            raw,
            command: command.unwrap(),
            params,
            prefix,
            nick,
            user,
            host,
        }
    }

    pub fn parse_replace(raw: String) -> Self {
        let mut prefix: Option<(u16, u16)> = None;
        let mut nick: Option<(u16, u16)> = None;
        let mut user: Option<(u16, u16)> = None;
        let mut host: Option<(u16, u16)> = None;
        let mut command: Option<(u16, u16)> = None;
        let mut params: Vec<(u16, u16)> = Vec::new();

        enum State {
            Initial,
            PrefixNick { begin: u16 },
            PrefixUser { begin: u16, begin_prefix: u16 },
            PrefixHost { begin: u16, begin_prefix: u16 },
            Command { begin: u16 },
            Params,
            ParamsMiddle { begin: u16 },
            ParamsTrailing { begin: u16 },
        }

        let mut state = State::Initial;

        for (i, b) in raw.bytes().enumerate() {
            // if b == b'\r' || b == b'\n' {
            //     break;
            // }
            match state {
                State::Initial => match b {
                    b':' => {
                        state = State::PrefixNick {
                            begin: i as u16 + 1,
                        }
                    }
                    _ => {
                        state = State::Command { begin: i as u16 };
                    }
                },
                State::PrefixNick { begin } => match b {
                    b'!' => {
                        nick.replace((begin, i as u16));

                        state = State::PrefixUser {
                            begin: i as u16 + 1,
                            begin_prefix: begin,
                        };
                    }
                    b'@' => {
                        nick.replace((begin, i as u16));

                        state = State::PrefixHost {
                            begin: i as u16 + 1,
                            begin_prefix: begin,
                        };
                    }
                    b' ' => {
                        nick.replace((begin, i as u16));
                        prefix.replace((begin, i as u16));

                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                    }
                    _ => {}
                },
                State::PrefixUser {
                    begin,
                    begin_prefix,
                } => match b {
                    b'@' => {
                        user.replace((begin, i as u16));

                        state = State::PrefixHost {
                            begin: i as u16 + 1,
                            begin_prefix,
                        };
                    }
                    b' ' => {
                        user.replace((begin, i as u16));
                        prefix.replace((begin_prefix, i as u16));

                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                    }
                    _ => {}
                },
                State::PrefixHost {
                    begin,
                    begin_prefix,
                } => match b {
                    b' ' => {
                        host.replace((begin, i as u16));
                        prefix.replace((begin_prefix, i as u16));

                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                    }
                    _ => {}
                },
                State::Command { begin } => match b {
                    b' ' => {
                        command.replace((begin, i as u16));

                        state = State::Params;
                    }
                    _ => {}
                },
                State::Params => match b {
                    b' ' => {}
                    b':' => {
                        state = State::ParamsTrailing {
                            begin: i as u16 + 1,
                        };
                        // break;
                    }
                    _ => {
                        state = State::ParamsMiddle { begin: i as u16 };
                    }
                },
                State::ParamsMiddle { begin } => match b {
                    b' ' => {
                        params.push((begin, i as u16));

                        state = State::Params;
                    }
                    _ => {}
                },
                State::ParamsTrailing { begin: _ } => match b {
                    _ => {}
                },
            }
        }

        match state {
            State::Command { begin } => {
                command.replace((begin, raw.len() as u16));
            }
            State::ParamsTrailing { begin } => {
                params.push((begin, raw.len() as u16));
            }
            State::ParamsMiddle { begin } => {
                params.push((begin, raw.len() as u16));
            }
            _ => {}
        }

        Self {
            raw,
            command: command.unwrap(),
            params,
            prefix,
            nick,
            user,
            host,
        }
    }

    pub fn parse_iter(raw: String) -> Self {
        // let mut rawSlice:Option<(u16,u16)> = None;
        let mut prefix: Option<(u16, u16)> = None;
        let mut nick: Option<(u16, u16)> = None;
        let mut user: Option<(u16, u16)> = None;
        let mut host: Option<(u16, u16)> = None;
        let mut command: Option<(u16, u16)> = None;
        let mut params: Vec<(u16, u16)> = Vec::new();

        enum State {
            PrefixNick { begin: u16 },
            PrefixUser { begin: u16, begin_prefix: u16 },
            PrefixHost { begin: u16, begin_prefix: u16 },
            Command { begin: u16 },
            Params,
            ParamsMiddle { begin: u16 },
            ParamsTrailing { begin: u16 },
        }

        let mut state = State::Command { begin: 0 };
        let mut iter = raw.bytes().until(b'\n').until(b'\r').enumerate();
        if let Some((_i, b)) = iter.next() {
            if b == b':' {
                state = State::PrefixNick { begin: 1 };
            }
        } else {
            panic!();
        }

        match state {
            State::PrefixNick { begin } => {
                while let Some((i, b)) = iter.next() {
                    if b == b'!' {
                        nick.replace((begin, i as u16));
                        state = State::PrefixUser {
                            begin: i as u16 + 1,
                            begin_prefix: begin,
                        };
                        break;
                    } else if b == b' ' {
                        nick.replace((begin, i as u16));
                        prefix.replace((begin, i as u16));
                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                        break;
                    }
                }
            }
            _ => {}
        }

        match state {
            State::PrefixUser {
                begin,
                begin_prefix,
            } => {
                while let Some((i, b)) = iter.next() {
                    if b == b'@' {
                        user.replace((begin, i as u16));
                        state = State::PrefixHost {
                            begin: i as u16 + 1,
                            begin_prefix,
                        };
                        break;
                    } else if b == b' ' {
                        user.replace((begin, i as u16));
                        prefix.replace((begin_prefix, i as u16));
                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                        break;
                    }
                }
            }
            _ => {}
        }

        match state {
            State::PrefixHost {
                begin,
                begin_prefix,
            } => {
                while let Some((i, b)) = iter.next() {
                    if b == b' ' {
                        host.replace((begin, i as u16));
                        prefix.replace((begin_prefix, i as u16));
                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                        break;
                    }
                }
            }
            _ => {}
        }

        match state {
            State::Command { begin } => {
                while let Some((i, b)) = iter.next() {
                    if b == b' ' {
                        command.replace((begin, i as u16));
                        state = State::Params;
                        break;
                    }
                }
            }
            _ => {}
        }

        while let Some((i, b)) = iter.next() {
            match state {
                State::Params => {
                    if b == b':' {
                        state = State::ParamsTrailing {
                            begin: i as u16 + 1,
                        };
                    } else {
                        state = State::ParamsMiddle { begin: i as u16 };
                    }
                }
                State::ParamsMiddle { begin } => {
                    while let Some((i, b)) = iter.next() {
                        if b == b' ' {
                            params.push((begin, i as u16));
                            state = State::Params;
                            break;
                        }
                    }
                    // params.push((begin, raw.len() as u16));
                }
                State::ParamsTrailing { begin } => {
                    //while let Some(_) = iter.next() {}
                    iter.for_each(drop);
                    params.push((begin, raw.len() as u16));
                    break;
                }
                _ => {}
            }
        }

        match state {
            State::Command { begin } => {
                command.replace((begin, raw.len() as u16));
            }
            State::ParamsMiddle { begin } => {
                params.push((begin, raw.len() as u16));
            }
            _ => {}
        }

        Self {
            raw,
            command: command.unwrap(),
            params,
            prefix,
            nick,
            user,
            host,
        }
    }

    pub fn parse_for_iter(raw: String) -> Self {
        let mut prefix: Option<(u16, u16)> = None;
        let mut nick: Option<(u16, u16)> = None;
        let mut user: Option<(u16, u16)> = None;
        let mut host: Option<(u16, u16)> = None;
        let mut command: Option<(u16, u16)> = None;
        let mut params: Vec<(u16, u16)> = Vec::new();

        #[derive(Debug)]
        enum State {
            PrefixNick { begin: u16 },
            PrefixUser { begin: u16, begin_prefix: u16 },
            PrefixHost { begin: u16, begin_prefix: u16 },
            Command { begin: u16 },
            Params,
            ParamsMiddle { begin: u16 },
            ParamsTrailing { begin: u16 },
        }

        let mut state = State::Command { begin: 0 };
        let mut iter = raw.bytes().until(b'\n').until(b'\r').enumerate();

        if let Some((_i, b)) = iter.next() {
            if b == b':' {
                state = State::PrefixNick { begin: 1 };
            }
        } else {
            panic!();
        }

        match state {
            State::PrefixNick { begin } => {
                for (i, b) in iter.by_ref() {
                    if b == b'!' {
                        nick.replace((begin, i as u16));
                        state = State::PrefixUser {
                            begin: i as u16 + 1,
                            begin_prefix: begin,
                        };
                        break;
                    } else if b == b' ' {
                        nick.replace((begin, i as u16));
                        prefix.replace((begin, i as u16));
                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                        break;
                    }
                }
            }
            _ => {}
        }

        match state {
            State::PrefixUser {
                begin,
                begin_prefix,
            } => {
                for (i, b) in iter.by_ref() {
                    if b == b'@' {
                        user.replace((begin, i as u16));
                        state = State::PrefixHost {
                            begin: i as u16 + 1,
                            begin_prefix,
                        };
                        break;
                    } else if b == b' ' {
                        user.replace((begin, i as u16));
                        prefix.replace((begin_prefix, i as u16));
                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                        break;
                    }
                }
            }
            _ => {}
        }

        match state {
            State::PrefixHost {
                begin,
                begin_prefix,
            } => {
                for (i, b) in iter.by_ref() {
                    if b == b' ' {
                        host.replace((begin, i as u16));
                        prefix.replace((begin_prefix, i as u16));
                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                        break;
                    }
                }
            }
            _ => {}
        }

        match state {
            State::Command { begin } => {
                for (i, b) in iter.by_ref() {
                    if b == b' ' {
                        command.replace((begin, i as u16));
                        state = State::Params;
                        break;
                    }
                }
            }
            _ => {}
        }

        while let Some((i, b)) = iter.next() {
            match state {
                State::Params => {
                    if b == b':' {
                        state = State::ParamsTrailing {
                            begin: i as u16 + 1,
                        };
                    } else {
                        state = State::ParamsMiddle { begin: i as u16 };
                    }
                }
                State::ParamsMiddle { begin } => {
                    for (i, b) in iter.by_ref() {
                        if b == b' ' {
                            params.push((begin, i as u16));
                            state = State::Params;
                            break;
                        }
                    }
                    // params.push((begin, raw.len() as u16));
                }
                State::ParamsTrailing { begin } => {
                    //while let Some(_) = iter.next() {}
                    iter.for_each(drop);
                    params.push((begin, raw.len() as u16));
                    break;
                }
                _ => {}
            }
        }

        match state {
            State::Command { begin } => {
                command.replace((begin, raw.len() as u16));
            }
            State::ParamsMiddle { begin } => {
                params.push((begin, raw.len() as u16));
            }
            _ => {}
        }

        Self {
            raw,
            command: command.unwrap(),
            params,
            prefix,
            nick,
            user,
            host,
        }
    }

    pub fn parse_foreach(raw: String) -> Self {
        let mut prefix: Option<(u16, u16)> = None;
        let mut nick: Option<(u16, u16)> = None;
        let mut user: Option<(u16, u16)> = None;
        let mut host: Option<(u16, u16)> = None;
        let mut command: Option<(u16, u16)> = None;
        let mut params: Vec<(u16, u16)> = Vec::new();

        enum State {
            Initial,
            PrefixNick { begin: u16 },
            PrefixUser { begin: u16, begin_prefix: u16 },
            PrefixHost { begin: u16, begin_prefix: u16 },
            Command { begin: u16 },
            Params,
            ParamsMiddle { begin: u16 },
            ParamsTrailing { begin: u16 },
        }

        let mut state = State::Initial;

        raw.bytes().enumerate().for_each(|(i, b)| {
            // if b == b'\r' || b == b'\n' {
            //     break;
            // }
            match state {
                State::Initial => match b {
                    b':' => {
                        state = State::PrefixNick {
                            begin: i as u16 + 1,
                        }
                    }
                    _ => {
                        state = State::Command { begin: i as u16 };
                    }
                },
                State::PrefixNick { begin } => match b {
                    b'!' => {
                        nick = Some((begin, i as u16));

                        state = State::PrefixUser {
                            begin: i as u16 + 1,
                            begin_prefix: begin,
                        };
                    }
                    b'@' => {
                        nick = Some((begin, i as u16));

                        state = State::PrefixHost {
                            begin: i as u16 + 1,
                            begin_prefix: begin,
                        };
                    }
                    b' ' => {
                        nick = Some((begin, i as u16));
                        prefix = Some((begin, i as u16));

                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                    }
                    _ => {}
                },
                State::PrefixUser {
                    begin,
                    begin_prefix,
                } => match b {
                    b'@' => {
                        user = Some((begin, i as u16));

                        state = State::PrefixHost {
                            begin: i as u16 + 1,
                            begin_prefix,
                        };
                    }
                    b' ' => {
                        user = Some((begin, i as u16));
                        prefix = Some((begin_prefix, i as u16));

                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                    }
                    _ => {}
                },
                State::PrefixHost {
                    begin,
                    begin_prefix,
                } => match b {
                    b' ' => {
                        host = Some((begin, i as u16));
                        prefix = Some((begin_prefix, i as u16));

                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                    }
                    _ => {}
                },
                State::Command { begin } => match b {
                    b' ' => {
                        command = Some((begin, i as u16));

                        state = State::Params;
                    }
                    _ => {}
                },
                State::Params => match b {
                    b' ' => {}
                    b':' => {
                        state = State::ParamsTrailing {
                            begin: i as u16 + 1,
                        };
                        // break;
                    }
                    _ => {
                        state = State::ParamsMiddle { begin: i as u16 };
                    }
                },
                State::ParamsMiddle { begin } => match b {
                    b' ' => {
                        params.push((begin, i as u16));

                        state = State::Params;
                    }
                    _ => {}
                },
                State::ParamsTrailing { begin: _ } => match b {
                    _ => {}
                },
            }
        });

        match state {
            State::Command { begin } => {
                command = Some((begin, raw.len() as u16));
            }
            State::ParamsTrailing { begin } => {
                params.push((begin, raw.len() as u16));
            }
            State::ParamsMiddle { begin } => {
                params.push((begin, raw.len() as u16));
            }
            _ => {}
        }

        Self {
            raw,
            command: command.unwrap(),
            params,
            prefix,
            nick,
            user,
            host,
        }
    }

    pub fn parse_loop(raw: String) -> Self {
        let mut prefix: Option<(u16, u16)> = None;
        let mut nick: Option<(u16, u16)> = None;
        let mut user: Option<(u16, u16)> = None;
        let mut host: Option<(u16, u16)> = None;
        let mut command: Option<(u16, u16)> = None;
        let mut params: Vec<(u16, u16)> = Vec::new();

        enum State {
            Initial,
            PrefixNick { begin: u16 },
            PrefixUser { begin: u16, begin_prefix: u16 },
            PrefixHost { begin: u16, begin_prefix: u16 },
            Command { begin: u16 },
            Params,
            ParamsMiddle { begin: u16 },
            ParamsTrailing { begin: u16 },
        }

        let mut state = State::Initial;

        let mut i = 0;
        let bytes = raw.as_bytes();
        loop {
            if i >= raw.len() {
                break;
            }
            let b = bytes[i];
            i += 1;
            if b == b'\r' || b == b'\n' {
                break;
            }
            match state {
                State::Initial => match b {
                    b':' => {
                        state = State::PrefixNick {
                            begin: i as u16 + 1,
                        }
                    }
                    _ => {
                        state = State::Command { begin: i as u16 };
                    }
                },
                State::PrefixNick { begin } => match b {
                    b'!' => {
                        nick = Some((begin, i as u16));

                        state = State::PrefixUser {
                            begin: i as u16 + 1,
                            begin_prefix: begin,
                        };
                    }
                    b'@' => {
                        nick = Some((begin, i as u16));

                        state = State::PrefixHost {
                            begin: i as u16 + 1,
                            begin_prefix: begin,
                        };
                    }
                    b' ' => {
                        nick = Some((begin, i as u16));
                        prefix = Some((begin, i as u16));

                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                    }
                    _ => {}
                },
                State::PrefixUser {
                    begin,
                    begin_prefix,
                } => match b {
                    b'@' => {
                        user = Some((begin, i as u16));

                        state = State::PrefixHost {
                            begin: i as u16 + 1,
                            begin_prefix,
                        };
                    }
                    b' ' => {
                        user = Some((begin, i as u16));
                        prefix = Some((begin_prefix, i as u16));

                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                    }
                    _ => {}
                },
                State::PrefixHost {
                    begin,
                    begin_prefix,
                } => match b {
                    b' ' => {
                        host = Some((begin, i as u16));
                        prefix = Some((begin_prefix, i as u16));

                        state = State::Command {
                            begin: i as u16 + 1,
                        };
                    }
                    _ => {}
                },
                State::Command { begin } => match b {
                    b' ' => {
                        command = Some((begin, i as u16));

                        state = State::Params;
                    }
                    _ => {}
                },
                State::Params => match b {
                    b' ' => {}
                    b':' => {
                        state = State::ParamsTrailing {
                            begin: i as u16 + 1,
                        };
                        // break;
                    }
                    _ => {
                        state = State::ParamsMiddle { begin: i as u16 };
                    }
                },
                State::ParamsMiddle { begin } => match b {
                    b' ' => {
                        params.push((begin, i as u16));

                        state = State::Params;
                    }
                    _ => {}
                },
                State::ParamsTrailing { begin: _ } => match b {
                    _ => {}
                },
            }
        }

        match state {
            State::Command { begin } => {
                command = Some((begin, raw.len() as u16));
            }
            State::ParamsTrailing { begin } => {
                params.push((begin, raw.len() as u16));
            }
            State::ParamsMiddle { begin } => {
                params.push((begin, raw.len() as u16));
            }
            _ => {}
        }

        Self {
            raw,
            command: command.unwrap(),
            params,
            prefix,
            nick,
            user,
            host,
        }
    }
}

// #[derive(Debug, Clone)]
// pub struct BaseMsg {
//     raw: String,
//     command: Option<(u8, u8)>,
//     prefix: Option<(u8, u8)>,
//     nick: Option<(u8, u8)>,
// }

// impl BaseMsg {
//     pub fn new(
//         raw: String,
//         command: Option<(u8, u8)>,
//         prefix: Option<(u8, u8)>,
//         nick: Option<(u8, u8)>,
//     ) -> Self {
//         Self {
//             raw,
//             command,
//             prefix,
//             nick,
//         }
//     }

//     pub fn parse(msg: String) -> Self {
//         let command: Option<(u8, u8)> = None;
//         let prefix: Option<(u8, u8)> = None;
//         let nick: Option<(u8, u8)> = None;

//         Self {
//             raw: msg,
//             command,
//             prefix,
//             nick,
//         }
//     }

//     #[inline]
//     pub fn command(&self) -> Option<String> {
//         match self.command {
//             Some((begin, end)) => Some(self.raw[begin as usize..end as usize].to_string()),
//             None => None,
//         }
//     }

//     pub fn prefix(&self) -> Option<String> {
//         match self.prefix {
//             Some((begin, end)) => Some(self.raw[begin as usize..end as usize].to_string()),
//             None => None,
//         }
//     }
// }

// #[derive(Debug, Clone)]
// pub struct PRIVMSG {
//     pub raw: String,
//     pub prefix: Option<(*const u8, usize)>,
//     pub command: *const str,
// }

// pub trait Msg {
//     fn raw(&self) -> &String;
//     // fn command(&self) -> Option<String>;
// }

// impl Msg for PRIVMSG {
//     fn raw(&self) -> &String {
//         &self.raw
//     }
//     // fn command(&self) -> Option<String> {
//     //     if let Some((start, len)) = self.prefix {
//     //         let slice = slice::from_raw_parts(start, len);
//     //         Some(str::from_utf8_unchecked(slice).to_string())
//     //     } else {
//     //         None
//     //     }
//     // }
// }

// // #[derive(Debug, Clone)]
// // pub enum Message {
// //     PRIVMSG(PRIVMSG),
// //     CUSTOM(String, PRIVMSG),
// //     NOOP,
// // }

// pub fn from(msg: &String) -> BaseMsg {
//     // <message>  ::= [':' <prefix> <SPACE> ] <command> <params> <crlf>
//     // <prefix>   ::= <servername> | <nick> [ '!' <user> ] [ '@' <host> ]
//     // <command>  ::= <letter> { <letter> } | <number> <number> <number>
//     // <SPACE>    ::= ' ' { ' ' }
//     // <params>   ::= <SPACE> [ ':' <trailing> | <middle> <params> ]

//     // <middle>   ::= <Any *non-empty* sequence of octets not including SPACE
//     //                or NUL or CR or LF, the first of which may not be ':'>
//     // <trailing> ::= <Any, possibly *empty*, sequence of octets not including
//     //                  NUL or CR or LF>

//     // <crlf>     ::= CR LF

//     let raw = msg.clone();
//     let mut prefix: Option<(u8, u8)> = None;
//     let command: &str;
//     let params: Vec<&str>;

//     let sliceBegin: usize;
//     let sliceEnd: usize;
//     for c in msg.chars() {
//         match c {
//             ':' => {
//                 let slice = &msg[1..3];
//                 prefix = Some((1, 3));
//             }
//             _ => {}
//         }
//     }

//     BaseMsg::new(raw, None, prefix, None)
// }
// impl Message{
//     pub fn from_str(msg: &str) -> Message{
//         let mut msg = msg.to_string();
//         let mut split = msg.split(" ");
//         let command = split.next().unwrap();
//         let mut params = split.collect::<Vec<&str>>();
//         match command{
//             "PING" => Message::Ping(params[0].to_string()),
//             "PONG" => Message::Pong(params[0].to_string()),
//             "PRIVMSG" => {
//                 let nick = params[0].to_string();
//                 let user = params[1].to_string();
//                 let host = params[2].to_string();
//                 let target = params[3].to_string();
//                 let message = params[4..].join(" ");
//                 Message::PRIVMSG(PRIVMSG{nick, user, host, target, message})
//             },
//             "JOIN" => Message::Join(params[0].to_string()),
//             "PART" => Message::Part(params[0].to_string()),
//             _ => Message::Raw(msg),
//         }
//     }
// }

// pub struct Message {
//     prefix: Option<String>,
//     command: String,
//     params: Vec<String>,
// }

// impl Message {
//     pub fn parse(message:&String ) -> Result<Message,()> {
//         return Ok(Message {
//             prefix: None,
//             command: String::from(""),
//             params: Vec::new(),
//         });

//         //Err(())
//     }
// }

// struct UntilLinebreak {
//     inner: dyn Iterator<Item = u8>,
// }

// impl Iterator for UntilLinebreak {
//     type Item = u8;

//     fn next(&mut self) -> Option<Self::Item> {
//         let n = self.inner.next();
//         match n {
//             Some(b'\r') => None,
//             Some(b'\n') => None,
//             _ => n,
//         }
//     }
// }
mod util;
use util::UntilExt;

#[cfg(test)]
mod tests {
    use super::*;
    extern crate test;

    #[test]
    fn test_parse() {
        let msg = ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
        let msg = ParsedMessage::parse_iter(msg);

        assert_eq!(
            msg.raw,
            ":irc.example.com 001 test :Welcome to the Internet Relay Network"
        );
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
            ":<nick>!<user>@<user>.tmi.twitch.tv PRIVMSG #<channel> :This is a sample message"
                .to_string();
        let msg = ParsedMessage::parse_iter(msg);

        assert_eq!(
            msg.raw,
            ":<nick>!<user>@<user>.tmi.twitch.tv PRIVMSG #<channel> :This is a sample message"
        );
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

    // #[test]
    // fn test_parse_linebreak() {
    //     let msg =
    //         ":irc.example.com 001 test :Welcome to the Internet Relay Network\r\n".to_string();
    //     let msg = ParsedMessage::parse_iter(msg);

    //     assert_eq!(
    //         msg.raw,
    //         ":irc.example.com 001 test :Welcome to the Internet Relay Network"
    //     );
    //     assert_eq!(msg.command(), "001");
    //     assert_eq!(msg.prefix(), Some("irc.example.com".to_string()));
    //     assert_eq!(
    //         msg.params(),
    //         vec!["test", "Welcome to the Internet Relay Network"]
    //     );
    // }

    // #[bench]
    // fn bench_parse(b: &mut test::Bencher) {
    //     let msg = ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
    //     b.iter(|| ParsedMessage::parse(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_small(b: &mut test::Bencher) {
    //     let msg = "PING".to_string();
    //     b.iter(|| ParsedMessage::parse(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_with_prefix(b: &mut test::Bencher) {
    //     let msg =
    //         ":<nick>!<user>@<user>.tmi.twitch.tv PRIVMSG #<channel> :This is a sample message"
    //             .to_string();
    //     b.iter(|| ParsedMessage::parse(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_long_trailing(b: &mut test::Bencher) {
    //     let front = ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
    //     let back = "_".repeat(448);
    //     let msg = format!("{}{}", front, back);

    //     assert_eq!(msg.len(), 512);

    //     b.iter(|| ParsedMessage::parse(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_long_nick(b: &mut test::Bencher) {
    //     let front = ":".to_string();
    //     let nick = "_".repeat(512 - 6);
    //     let back = " PING".to_string();
    //     let msg = format!("{}{}{}", front, nick, back);

    //     assert_eq!(msg.len(), 512);

    //     b.iter(|| ParsedMessage::parse(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_replace(b: &mut test::Bencher) {
    //     let msg = ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
    //     b.iter(|| ParsedMessage::parse_replace(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_small_replace(b: &mut test::Bencher) {
    //     let msg = "PING".to_string();
    //     b.iter(|| ParsedMessage::parse_replace(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_long_nick_replace(b: &mut test::Bencher) {
    //     let front = ":".to_string();
    //     let nick = "_".repeat(512 - 6);
    //     let back = " PING".to_string();
    //     let msg = format!("{}{}{}", front, nick, back);

    //     assert_eq!(msg.len(), 512);

    //     b.iter(|| ParsedMessage::parse_replace(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_long_trailing_replace(b: &mut test::Bencher) {
    //     let front = ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
    //     let back = "_".repeat(448);
    //     let msg = format!("{}{}", front, back);

    //     assert_eq!(msg.len(), 512);

    //     b.iter(|| ParsedMessage::parse_replace(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_foreach(b: &mut test::Bencher) {
    //     let msg = ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
    //     b.iter(|| ParsedMessage::parse_foreach(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_small_foreach(b: &mut test::Bencher) {
    //     let msg = "PING".to_string();
    //     b.iter(|| ParsedMessage::parse_foreach(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_long_nick_foreach(b: &mut test::Bencher) {
    //     let front = ":".to_string();
    //     let nick = "_".repeat(512 - 6);
    //     let back = " PING".to_string();
    //     let msg = format!("{}{}{}", front, nick, back);

    //     assert_eq!(msg.len(), 512);

    //     b.iter(|| ParsedMessage::parse_foreach(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_long_trailing_foreach(b: &mut test::Bencher) {
    //     let front = ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
    //     let back = "_".repeat(448);
    //     let msg = format!("{}{}", front, back);

    //     assert_eq!(msg.len(), 512);

    //     b.iter(|| ParsedMessage::parse_foreach(msg.clone()));
    // }

    mod iter {
        use super::*;

        #[bench]
        fn bench_parse_usual(b: &mut test::Bencher) {
            let msg =
                ":irc.example.com 001 test :Welcome to the Internet Relay Network\r\n".to_string();
            b.iter(|| ParsedMessage::parse_iter(msg.clone()));
        }

        #[bench]
        fn bench_parse_small(b: &mut test::Bencher) {
            let msg = "PING \r\n".to_string();
            b.iter(|| ParsedMessage::parse_iter(msg.clone()));
        }

        #[bench]
        fn bench_parse_long_nick(b: &mut test::Bencher) {
            let front = ":".to_string();
            let nick = "_".repeat(512 - 9);
            let back = " PING ".to_string();
            let msg = format!("{}{}{}\r\n", front, nick, back);

            assert_eq!(msg.len(), 512);

            b.iter(|| ParsedMessage::parse_iter(msg.clone()));
        }

        #[bench]
        fn bench_parse_long_trailing(b: &mut test::Bencher) {
            let front =
                ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
            let back = "_".repeat(446);
            let msg = format!("{}{}\r\n", front, back);

            assert_eq!(msg.len(), 512);

            b.iter(|| ParsedMessage::parse_iter(msg.clone()));
        }
    }

    // #[bench]
    // fn bench_parse_for_iter(b: &mut test::Bencher) {
    //     let msg = ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
    //     b.iter(|| ParsedMessage::parse_for_iter(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_small_for_iter(b: &mut test::Bencher) {
    //     let msg = "PING".to_string();
    //     b.iter(|| ParsedMessage::parse_for_iter(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_long_nick_for_iter(b: &mut test::Bencher) {
    //     let front = ":".to_string();
    //     let nick = "_".repeat(512 - 6);
    //     let back = " PING".to_string();
    //     let msg = format!("{}{}{}", front, nick, back);

    //     assert_eq!(msg.len(), 512);

    //     b.iter(|| ParsedMessage::parse_for_iter(msg.clone()));
    // }

    // #[bench]
    // fn bench_parse_long_trailing_for_iter(b: &mut test::Bencher) {
    //     let front = ":irc.example.com 001 test :Welcome to the Internet Relay Network".to_string();
    //     let back = "_".repeat(448);
    //     let msg = format!("{}{}", front, back);

    //     assert_eq!(msg.len(), 512);

    //     b.iter(|| ParsedMessage::parse_for_iter(msg.clone()));
    // }

    #[test]
    fn test_until() {
        let vec = vec![1, 2, 3];
        let mut iter = vec.into_iter();
        let mut iter_ref = iter.by_ref().until(2);
        assert_eq!(iter_ref.next(), Some(1));
        assert_eq!(iter_ref.next(), None);
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_until_until() {
        let vec = vec![1, 2, 3];
        let mut iter = vec.into_iter();
        let mut iter_ref = iter.by_ref().until(2).until(1);
        assert_eq!(iter_ref.next(), None);
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }
    #[test]
    fn test_until_until2() {
        let vec = vec![1, 2, 3];
        let mut iter = vec.into_iter();
        let mut iter_ref = iter.by_ref().until(1).until(2);
        assert_eq!(iter_ref.next(), None);
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }

    #[bench]
    fn bench_until_mapped2(b: &mut test::Bencher) {
        let mut msg = "_".repeat(512);
        msg.push('\n');
        msg.push_str(&"_".repeat(511));
        b.iter(|| {
            let mut i1 = msg.bytes();
            let mut iter = i1.by_ref().until(b'\n').until(b'\r');
            while let Some(_) = iter.next() {
                // if c == b'\n' {
                //     break;
                // }
            }
            i1
            // println!("{:?}", iter.collect::<Vec<_>>());
        });
    }

    #[bench]
    fn bench_until_mapped(b: &mut test::Bencher) {
        let mut msg = "_".repeat(512);
        msg.push('\n');
        msg.push_str(&"_".repeat(511));
        b.iter(|| {
            let mut i1 = msg.bytes();
            let mut iter = i1.by_ref().until(b'\n');
            while let Some(_) = iter.next() {
                // if c == b'\n' {
                //     break;
                // }
            }
            i1
            // println!("{:?}", iter.collect::<Vec<_>>());
        });
    }

    #[bench]
    fn bench_until_raw(b: &mut test::Bencher) {
        let mut msg = "_".repeat(512);
        msg.push('\n');
        msg.push_str(&"_".repeat(511));
        b.iter(|| {
            let mut iter = msg.bytes();
            while let Some(c) = iter.next() {
                if c == b'\n' || c == b'\r' {
                    break;
                }
            }
            iter
            // println!("{:?}", iter.collect::<Vec<_>>());
        });
    }
}
