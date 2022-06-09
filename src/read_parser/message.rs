use smallvec::SmallVec;

use crate::message::prelude::*;

pub struct ParsedMessage {
    pub(crate) raw: String,
    pub(crate) command: (u16, u16),
    pub(crate) params: SmallVec<[(u16, u16); 2]>,
    pub(crate) prefix: Option<(u16, u16)>,
    pub(crate) nick: Option<(u16, u16)>,
    pub(crate) user: Option<(u16, u16)>,
    pub(crate) host: Option<(u16, u16)>,
}

#[inline(always)]
fn unsafe_slice(raw: &String, range: &(u16, u16)) -> String {
    debug_assert!(raw.len() <= u16::MAX as usize);
    debug_assert!(range.0 <= range.1);
    debug_assert!(range.0 <= raw.len() as u16);
    debug_assert!(range.1 <= raw.len() as u16);
    unsafe {
        raw.get_unchecked(range.0 as usize..range.1 as usize)
            .to_string()
    }
}

impl Message for ParsedMessage {
    fn command(&self) -> String {
        unsafe_slice(&self.raw, &self.command)
    }
}

impl Parameterized for ParsedMessage {
    fn params(&self) -> Vec<String> {
        self.params
            .iter()
            .map(|p| unsafe_slice(&self.raw, p))
            .collect()
    }
}
impl Prefixed for ParsedMessage {
    fn prefix(&self) -> Option<String> {
        self.prefix.map(|ref p| unsafe_slice(&self.raw, p))
    }
    fn nick(&self) -> Option<String> {
        self.nick.map(|ref p| unsafe_slice(&self.raw, p))
    }
    fn user(&self) -> Option<String> {
        self.user.map(|ref p| unsafe_slice(&self.raw, p))
    }
    fn host(&self) -> Option<String> {
        self.host.map(|ref p| unsafe_slice(&self.raw, p))
    }
}

impl ParsedMessage {
    pub(crate) fn new() -> Self {
        Self {
            raw: String::new(),
            command: (0, 0),
            params: SmallVec::new(),
            prefix: None,
            nick: None,
            user: None,
            host: None,
        }
    }
}

impl std::fmt::Display for ParsedMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl std::fmt::Debug for ParsedMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("Message");

        debug_struct.field("command", &self.command());

        if let Some(prefix) = self.prefix() {
            debug_struct.field("prefix", &prefix);
        }

        if let Some(nick) = self.nick() {
            debug_struct.field("nick", &nick);
        }

        if let Some(user) = self.user() {
            debug_struct.field("user", &user);
        }

        if let Some(host) = self.host() {
            debug_struct.field("host", &host);
        }

        let params = self.params();
        if !params.is_empty() {
            debug_struct.field("params", &params);
        }

        debug_struct.finish()
    }
}
