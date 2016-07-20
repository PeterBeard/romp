/*
 * Copyright (C) 2016 Peter Beard
 * This file is part of Romp, the simple Rust STOMP server
 * Licensed under the GPLv3, see the LICENSE file for details
 */
#[macro_use]
extern crate log;

use std::net::TcpListener;
use std::thread;
use std::thread::JoinHandle;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

mod stomp;
use stomp::Frame;

mod client;
use client::handle_client;

const DEFAULT_HOST: &'static str = "127.0.0.1";
const DEFAULT_PORT: u32 = 61616;

use log::{LogRecord, LogLevel, LogMetadata};

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LogLevel::Info
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }
}

impl SimpleLogger {
    pub fn init() -> Result<(), log::SetLoggerError> {
        log::set_logger(|max_log_level| {
            max_log_level.set(log::LogLevelFilter::Info);
            Box::new(SimpleLogger)
        })
    }
}

// A client object containing the communication channel
struct Client {
    thread: JoinHandle<()>,
    tx: Sender<Frame>,
    rx: Receiver<Frame>,
}

impl Client {
    // Create a new client
    pub fn new(h: JoinHandle<()>, t: Sender<Frame>, r: Receiver<Frame>) -> Client {
        Client {
            thread: h,
            tx: t,
            rx: r,
        }
    }
}

fn main() {
    // Enable simple logging
    SimpleLogger::init();

    // Keep track of all our clients
    let mut clients: Vec<Client> = Vec::new();

    // Bind to our TCP port or panic
    let addr = format!("{}:{}", DEFAULT_HOST, DEFAULT_PORT);
    let listener = match TcpListener::bind(&addr[..]) {
        Ok(listener) => listener,
        Err(e) => panic!("Failed to bind to {}: {}", addr, e),
    };

    // Spin up a thread for TCP connection management
    let (client_tx, client_rx) = mpsc::channel::<Client>();
    thread::spawn(move || {
        tcp_listen(listener, client_tx);
    });
    info!("Started TCP listener thread.");
    
    // Handle frames from clients
    // TODO: this
    loop {
        // See if we have any new clients
        if let Ok(c) = client_rx.try_recv() {
            clients.push(c);
        }

        // Listen to and handle requests from the clients in turn
        for c in &mut clients {
            if let Ok(r) = c.rx.try_recv() {
                info!("Got request from client: {:?}", r);
            }
        }
    }
}

fn tcp_listen(listener: TcpListener, tx: Sender<Client>) {
    info!("Listening on {}", listener.local_addr().unwrap());
    // Handle incoming connections
    for stream in listener.incoming() {
        info!("Incoming stream.");
        match stream {
            Ok(stream) => {
                info!("Open stream from {}", stream.peer_addr().unwrap());
                let (client_tx, client_rx) = mpsc::channel::<Frame>();
                let (server_tx, server_rx) = mpsc::channel::<Frame>();

                let t = thread::spawn(move|| {
                    handle_client(stream, server_tx, client_rx);
                });
                let c = Client::new(t, client_tx, server_rx);
                // Send the client back to the main thread
                tx.send(c).unwrap();
            }
            Err(e) => {
                error!("Error in incoming stream: {}", e);
            }
        }
        info!("Done handling incoming stream.");
    }
}
