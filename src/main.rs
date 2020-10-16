use std::net::{TcpListener, TcpStream};
use std::{
    io::{Read, Write},
    str::from_utf8,
};

use keri::{event_message::parse::signed_message, state::IdentifierState};
mod log_state;

#[derive(Clone)]
struct Instance {
    address: String,
    log: log_state::LogState,
    bob_state: IdentifierState,
}

impl Instance {
    fn new(adr: String, log: log_state::LogState, bob_state: IdentifierState) -> Self {
        Instance {
            address: adr,
            log: log,
            bob_state: bob_state,
        }
    }

    // Server
    fn listen(&mut self) {
        let listener = TcpListener::bind(self.address.clone()).unwrap();
        println!("Server listening {}", self.address.clone());
        // accept connections and process them
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    self.handle_connection(stream);
                }
                Err(e) => {
                    // connection failed
                    println!("Error: {}", e);
                }
            }
        }
        // close the socket server
        drop(listener);
    }

    fn handle_connection(&mut self, mut stream: TcpStream) {
        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(size) => {
                // Deserialize signed bob's msg.
                let msg = signed_message(from_utf8(&buffer[..size]).unwrap())
                    .unwrap()
                    .1;
                // println!("Request: {:?}", msg);
                // process bob's message
                self.bob_state = self
                    .bob_state
                    .clone()
                    .verify_and_apply(&msg)
                    .expect("Can't verify bob's message.");
                let receipt_for_bob = self
                    .log
                    .make_rct(msg.event_message)
                    .expect("Can't make a receipt");

                // send receipt to bob.
                stream
                    .write(&receipt_for_bob.serialize().unwrap())
                    .expect("Can't write to buffer.");
                stream.flush().unwrap();
            }
            Err(_) => {}
        };
    }
}

fn main() {
    let mut alice = Instance::new(
        "0.0.0.0:3333".to_string(),
        log_state::LogState::new().unwrap(),
        IdentifierState::default(),
    );

    alice.listen();
}
