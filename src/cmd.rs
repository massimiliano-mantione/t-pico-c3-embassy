use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    Ok,
    Exit,
    Previous,
    Next,
    Plus,
    Minus,
}

pub static CMD: Signal<CriticalSectionRawMutex, Cmd> = Signal::new();

impl Cmd {
    pub fn from_serial_char(code: char) -> Option<Self> {
        match code {
            'Z' | 'z' => Some(Self::Ok),
            'X' | 'x' | ' ' => Some(Self::Exit),
            'S' | 's' => Some(Self::Previous),
            'A' | 'a' => Some(Self::Next),
            'P' | 'p' | 'W' | 'w' | '+' => Some(Self::Plus),
            'M' | 'm' | 'Q' | 'q' | '-' => Some(Self::Minus),
            _ => None,
        }
    }

    pub fn serial_char(&self) -> char {
        match self {
            Cmd::Ok => 'K',
            Cmd::Exit => 'X',
            Cmd::Previous => 'U',
            Cmd::Next => 'D',
            Cmd::Plus => 'P',
            Cmd::Minus => 'M',
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Cmd::Ok => "OK",
            Cmd::Exit => "EXIT",
            Cmd::Previous => "PREVIOUS",
            Cmd::Next => "NEXT",
            Cmd::Plus => "PLUS",
            Cmd::Minus => "MINUS",
        }
    }
}
