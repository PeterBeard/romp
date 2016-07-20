/*
 * Copyright (C) 2016 Peter Beard
 * This file is part of Romp, the simple Rust STOMP server
 * Licensed under the GPLv3, see the LICENSE file for details
 */

use std::net::TcpStream;
use std::io::Read;

use super::{Frame,StompCommand};

const ESCAPE_CHAR: u8 = 92;                 // Backslash is the escape character

// Parse a stream into a Frame object
pub fn parse_frame(stream: &TcpStream) -> Result<Frame, &'static str> {
    let mut cmd_buf: Vec<u8> = Vec::new();
    // The STOMP spec says to ignore trailing line breaks, but it's easier to ignore leading ones
    // Shouldn't make a difference though.

    let c_stream = stream.try_clone().unwrap();

    // Try to parse the command
    for b in (&c_stream).bytes() {
        // Add the byte to the command buffer
        match b {
            Ok(10) => {
                // Command ends on \n
                if cmd_buf.len() > 0 {
                    break;
                }
            },
            Ok(13) => { },
            Ok(b) => {
                cmd_buf.push(b);
            },
            Err(_) => {
                break;
            },
        }
    }
    // Parse the command
    let command = StompCommand::from_bytes(&cmd_buf[..]);

    let mut frame = Frame::new();
    match command {
        Some(c) => {
            frame.command = c;
        },
        None => {
            return Err("Invalid command");
        },
    }

    // Try to parse the header
    let mut eol_seen = 1;
    let mut key_buf: Vec<u8> = Vec::new();
    let mut value_buf: Vec<u8> = Vec::new();
    let mut found_colon = false;
    let mut escape = false;

    for byte in (&c_stream).bytes() {
        // Write the k/v pair on line break
        match byte {
            Ok(10) => {
                eol_seen += 1;
                // Once we hit two line breaks, the headers are over
                if eol_seen == 2 {
                    break;
                }

                if key_buf.len() > 0 {
                    // Malformed k/v pair
                    if !found_colon {
                        return Err("Failed to parse header.");
                    }

                    let key = String::from_utf8(key_buf).unwrap();
                    let value = String::from_utf8(value_buf).unwrap();
                    frame.header.set(&key, &value);
                }
                key_buf = Vec::new();
                value_buf = Vec::new();
                found_colon = false;
            },
            // Ignore \r
            Ok(13) => { },
            // Colon separates key from value
            Ok(58) => {
                found_colon = true;
            },
            // Start escape sequence
            Ok(ESCAPE_CHAR) => {
                escape = true;
            },
            // Add the byte to the correct buffer
            Ok(byte) => {
                eol_seen = 0;
                // Handle escape sequence -- returns an error immediately if it's invalid
                if escape {
                    match unescape(byte) {
                        Ok(byte) => {
                            if found_colon {
                                value_buf.push(byte);
                            } else {
                                key_buf.push(byte);
                            }
                        },
                        Err(e) => {
                            return Err(e);
                        }
                    };
                    escape = false;
                } else {
                    if found_colon {
                        value_buf.push(byte);
                    } else {
                        key_buf.push(byte);
                    }
                }
            },
            Err(_) => {
                break;
            },
        }
    }
    // If there weren't two line breaks after the header, the frame is malformed
    if eol_seen != 2 {
        return Err("Missing line breaks after header.");
    }

    // Try to parse the body
    // TODO: Implement content-length header
    let mut body_buf: Vec<u8> = Vec::new();

    for byte in (&c_stream).bytes() {
        match byte {
            // Body ends on NUL
            Ok(0) => {
                break;
            },
            Ok(b) => {
                body_buf.push(b);
            },
            Err(_) => {
                break;
            }
        }
    }
    let content = String::from_utf8(body_buf);
    match content {
        Ok(c) => {
            frame.body = c;
        },
        Err(_) => {
            return Err("Error decoding body.");
        }
    }

    // Only certain kinds of frames are allowed to have a body
    if frame.body.len() > 0 {
        if frame.command != StompCommand::Send && 
           frame.command != StompCommand::Message &&
           frame.command != StompCommand::Error {
            return Err("This type of frame may not have a body.");
        }
    }

    // Frame is parsed and valid
    Ok(frame)
}

// Unescape a byte
fn unescape(b: u8) -> Result<u8, &'static str> {
    // Carriage return
    match b {
        114 => Ok(13),
        110 => Ok(10),
        99 => Ok(58),
        92 => Ok(92),
        _ => Err("Invalid escape sequence")
    }
}
