/*
 * Copyright (C) 2016 Peter Beard
 * This file is part of Romp, the simple Rust STOMP server
 * Licensed under the GPLv3, see the LICENSE file for details
 */
use std::char;
use std::str;
use std::fmt::{self, Display};

// Possible STOMP commands
#[derive(Debug, PartialEq)]
pub enum StompCommand {
    // Client commands
    Stomp,
    Send,
    Subscribe,
    Unsubscribe,
    Ack,
    Nack,
    Begin,
    Commit,
    Abort,
    Disconnect,
    // Server commands
    Connected,
    Message,
    Receipt,
    Error,
}

impl StompCommand {
    // Create a StompCommand from a string
    pub fn from_string(string: &str) -> Option<StompCommand> {
        use self::StompCommand::*;
        match string {
            "SEND" => Some(Send),
            "SUBSCRIBE" => Some(Subscribe),
            "UNSUBSCRIBE" => Some(Unsubscribe),
            "BEGIN" => Some(Begin),
            "COMMIT" => Some(Commit),
            "ABORT" => Some(Abort),
            "ACK" => Some(Ack),
            "NACK" => Some(Nack),
            "DISCONNECT" => Some(Disconnect),
            "STOMP" => Some(Stomp),
            "CONNECT" => Some(Stomp),
            "CONNECTED" => Some(Connected),
            "MESSAGE" => Some(Message),
            "RECEIPT" => Some(Receipt),
            "ERROR" => Some(Error),
            _ => None,
        }
    }

    // Create a string from a StompCommand
    pub fn to_string(&self) -> &'static str {
        use self::StompCommand::*;
        match self {
            &Send => "SEND",
            &Subscribe => "SUBSCRIBE",
            &Unsubscribe => "UNSUBSCRIBE",
            &Begin => "BEGIN",
            &Commit => "COMMIT",
            &Abort => "ABORT",
            &Ack => "ACK",
            &Nack => "NACK",
            &Disconnect => "DISCONNECT",
            &Stomp => "STOMP",
            &Connected => "CONNECTED",
            &Message => "MESSAGE",
            &Receipt => "RECEIPT",
            &Error => "ERROR",
        }
    }

    // Create a StompCommand from a slice of bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<StompCommand> {
        let string = match str::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => "INVALID",
        };
        StompCommand::from_string(string)
    }
}

// Frame header
#[derive(Debug)]
pub struct Header {
    pub store: Vec<(String, String)>,
}

impl Header {
    pub fn new() -> Header {
        Header {
            store: Vec::new(),
        }
    }
    
    // Write the header as a string
    pub fn to_string(&self) -> String {
        let mut string = String::new();
        for pair in self.store.iter() {
            string = format!("{}{}:{}\r\n", string, pair.0, pair.1);
        }
        string
    }

    // Store a value
    pub fn set(&mut self, key: &str, value: &str) {
        self.store.push((String::from(key), String::from(value)));
    }

    // Retrieve a value
    pub fn get(&self, key: &str) -> Option<&String> {
        for pair in self.store.iter() {
            if pair.0 == key {
                return Some(&pair.1);
            }
        }
        None
    }

    // Determine whether the header contains the given key
    pub fn contains_key(&self, key: &str) -> bool {
        for pair in self.store.iter() {
            if pair.0 == key {
                return true;
            }
        }
        false
    }
}

// STOMP frame
#[derive(Debug)]
pub struct Frame {
    pub command: StompCommand,
    pub header: Header,
    pub body: String,
}

impl Frame {
    // Create a new frame -- defaults to error
    pub fn new() -> Frame {
        Frame {
            command: StompCommand::Error,
            header: Header::new(),
            body: String::new(),
        }
    }

    // Create a frame with the given command
    pub fn from_command(c: StompCommand) -> Frame {
        Frame {
            command: c,
            header: Header::new(),
            body: String::new(),
        }
    }

    // Create a frame with the given command and body
    // Automatically adds content-length header
    pub fn with_body(c: StompCommand, b: &str) -> Frame {
        let f = Frame {
            command: c,
            header: Header::new(),
            body: String::from(b),
        };
        f.header.set("content-length", &b.len().to_string()[..]);
    }

    // Represent a frame as a String
    pub fn to_string(&self) -> String {
        let c = self.command.to_string();
        let h = self.header.to_string();
        let nul = char::from_u32(0u32).unwrap();

        format!("{}\r\n{}\r\n\r\n{}{}", c, h, self.body, nul)
    }

    // Represent a frame as a vec of bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_string().into_bytes()
    }
}
