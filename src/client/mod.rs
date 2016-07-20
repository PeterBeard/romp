/*
 * Copyright (C) 2016 Peter Beard
 * This file is part of Romp, the simple Rust STOMP server
 * Licensed under the GPLv3, see the LICENSE file for details
 */
use std::net::{TcpStream, Shutdown};
use std::io::{Write, Read};
use std::time::Duration;

use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

use super::stomp::{Frame, StompCommand};
use super::stomp::{PROTO_VERS, SERVER_STR};
use super::stomp::parse::parse_frame;

// Service a client connection
pub fn handle_client(mut stream: TcpStream, tx: Sender<Frame>, rx: Receiver<Frame>) {
    // Set read/write timeouts
    let default_read_timeout = Some(Duration::new(10, 0));
    let default_write_timeout = Some(Duration::new(10, 0));

    stream.set_read_timeout(default_read_timeout);
    stream.set_write_timeout(default_write_timeout);

    let client_ip = stream.peer_addr().unwrap();
    info!("Started thread for client {:?}", client_ip);
    // Get the first frame from the client
    let request = parse_frame(&mut stream);

    let mut response = Frame::new();
    match request {
        Ok(r) => {
            info!("Got request {:?}", r);
            response = do_connect(&r);
            stream.write(&response.to_bytes()[..]).unwrap();
        },
        Err(e) => {
            response = Frame::with_body(StompCommand::Error, e);
            stream.write(&response.to_bytes()[..]).unwrap();
            return;
        },
    };
    
    // Listen until the client disconnects or something goes wrong
    loop {
        let request = parse_frame(&mut stream);
        
        let mut response = Frame::new();
        match request {
            Ok(r) => {
                info!("Got request {:?}", r);
                // send the request to the main thread for processing
                tx.send(r).unwrap();
                response = rx.recv().unwrap();
            },
            Err(e) => {
                response = Frame::with_body(StompCommand::Error, e);
            },
        };
        stream.write(&response.to_bytes()[..]).unwrap();
        // As soon as we write an error to the client, we have to close the connection
        if response.command == StompCommand::Error {
            info!("Error sent; closing connection");
            match stream.shutdown(Shutdown::Both) {
                Ok(_) => {
                    info!("Closed connection to client {:?}", client_ip);
                },
                Err(e) => {
                    debug!("Failed to close connection to client {:?}: {:?}", client_ip, e);
                },
            }
            break;
        }
    }
    info!("Ended thread for client {:?}", client_ip);
}

// Handle a new client
fn do_connect(r: &Frame) -> Frame {
    let mut response = Frame::new();
    // We expect all new connections to begin with a STOMP frame; anything else is invalid
    if r.command != StompCommand::Stomp {
        response = Frame::with_body(
            StompCommand::Error,
            "Invalid command; expected STOMP or CONNECT."
        );

    // Right type of frame; let's see if we can start talking
    } else {
        // We MUST have accept-version and host
        if !r.header.contains_key("accept-version") {
            response = Frame::with_body(
                StompCommand::Error,
                "Invalid frame; expected 'accept-version' header."
            );
        } else if !r.header.contains_key("host") {
            response = Frame::with_body(
                StompCommand::Error,
                "Invalid frame; expected 'host' header."
            );
        } else if r.header.get("accept-version").unwrap() != PROTO_VERS {
            response = Frame::with_body(
                StompCommand::Error,
                "Invalid protocol version."
            );
        // Respond with a CONNECTED frame
        } else {
            response = Frame::from_command(StompCommand::Connected);
            response.header.set("version", "1.2");
            response.header.set("server", SERVER_STR);
        }
    }
    response
}

