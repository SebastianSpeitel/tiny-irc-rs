pub trait Message {
    fn command(&self) -> String;
}

pub trait Prefixed {
    fn prefix(&self) -> Option<String>;
    fn nick(&self) -> Option<String>;
    fn user(&self) -> Option<String>;
    fn host(&self) -> Option<String>;
}

pub trait Parameterized {
    fn params(&self) -> Vec<String>;
}

pub trait IRCMessage: Message + Prefixed + Parameterized {}

impl<T> IRCMessage for T where T: Message + Prefixed + Parameterized {}
