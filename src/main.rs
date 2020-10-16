use std::net::{TcpListener, TcpStream};
use std::{
    io::{Read, Write},
    str::from_utf8,
};

use clap::App as clapapp;
use clap::Arg;
use keri::{
    event_message::parse::signed_event_stream, event_message::parse::signed_message,
    event_message::SignedEventMessage, state::IdentifierState,
};
mod log_state;

#[derive(Clone)]
struct Instance {
    log: log_state::LogState,
    bob_state: IdentifierState,
}

impl Instance {
    fn new(log: log_state::LogState, bob_state: IdentifierState) -> Self {
        Instance {
            log: log,
            bob_state: bob_state,
        }
    }

    // Server
    fn listen(&mut self, address: String) {
        let listener = TcpListener::bind(address.clone()).unwrap();
        println!("Server listening {}", address.clone());
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

                // Process Bob's message.
                let respond = match msg.event_message.event.event_data {
                    // if it's receipt message, verify it and add to sigs_map.
                    keri::event::event_data::EventData::Vrc(_) => {
                        self.log
                            .add_sig(self.bob_state.clone(), msg)
                            .expect("Can't verify receipt msg");
                        vec![]
                    }
                    // Otherwise respond with alice's last establishment message and receipt message.
                    _ => {
                        self.bob_state = self
                            .bob_state
                            .clone()
                            .verify_and_apply(&msg)
                            .expect("Can't verify bob's message.");
                        let receipt_for_bob = self
                            .log
                            .make_rct(msg.event_message)
                            .expect("Can't make a receipt");

                        // Respond with alice's lase establishment message, and receipt message.
                        let alice_last_est = self
                            .log
                            .log
                            .last()
                            .expect("There is no last alice's establishment event.");
                        let respond = [
                            alice_last_est.serialize().unwrap(),
                            receipt_for_bob.serialize().unwrap(),
                        ]
                        .concat();
                        respond
                    }
                };
                // Send response to bob.
                stream.write(&respond).expect("Can't write to buffer.");
                stream.flush().unwrap();
            }
            Err(_) => {}
        };
    }
    pub fn process_response(&mut self, response: Vec<SignedEventMessage>, address: String ){
        // Response is vec of SignedMessage. Handle it one by one. Apply establishment event
        // to other_state and handle receipt event.
        for sig_msg in response {
            match sig_msg.event_message.event.event_data {
                // If sig_msg is establishment event, update self.bob_state, and send receipt to bob.
                keri::event::event_data::EventData::Icp(_)
                | keri::event::event_data::EventData::Rot(_) => {
                    self.bob_state =
                        self.bob_state.clone().verify_and_apply(&sig_msg).expect("Can't verify bob's mesage.");
                    // Send receipt of message sig_msg.
                    let rcpt = self.log.make_rct(sig_msg.event_message);
                    send_to_other(address.to_string(), &rcpt.unwrap());
                }
                // If sig_msg is receipt event, verify it and add to sigs_map.
                keri::event::event_data::EventData::Vrc(_) => self
                    .log
                    .add_sig(self.bob_state.clone(), sig_msg)
                    .expect("Can't verify receipt msg."),
                _ => {}
            }
        }
    }
}

// Client
fn send_to_other(address: String, msg: &SignedEventMessage) -> Vec<SignedEventMessage> {
    let mut receipt_msgs: Vec<SignedEventMessage> = vec![];
    match TcpStream::connect(address.clone()) {
        Ok(mut stream) => {
            println!("Successfully connected with alice ({}) \n", address);

            // Serialize bob's signed message.
            let serialized_bob_msg = msg.serialize().unwrap();

            stream.write(&serialized_bob_msg).unwrap();
            println!(
                "Sent bob's msg, \n {:?} \n awaiting reply... \n",
                String::from_utf8(serialized_bob_msg.to_vec())
            );

            let mut buffer = [0; 1024];
            match stream.read(&mut buffer) {
                Ok(size) => {
                    receipt_msgs = signed_event_stream(from_utf8(&buffer[..size]).unwrap())
                        .unwrap()
                        .1;

                    println!("Replay: {:?} \n", receipt_msgs);
                    // println!("Replay: {:?} \n", from_utf8(&buffer[..size]).unwrap());
                }
                Err(e) => {
                    println!("Failed to receive data: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Failed to connect: {}", e);
        }
    }
    println!("Terminated.");
    receipt_msgs
}

fn main() {
    // Parse command line arguments.
    let matches = clapapp::new("get-command-line-args")
        .arg(
            Arg::with_name("host:port")
                .short('H'.to_string())
                .help("is the host:port of the other instance to connect to, ie: localhost:12345")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("is_server")
                .short('s'.to_string())
                .help("act as server")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("is_client")
                .short('c'.to_string())
                .help("act as client")
                .takes_value(false),
        )
        .get_matches();

    let address = matches.value_of("host:port").expect("Invalid socket");

    let mut alice = Instance::new(
        log_state::LogState::new().unwrap(),
        IdentifierState::default(),
    );

    // Decide if run as server or client.
    match (
        matches.is_present("is_server"),
        matches.is_present("is_client"),
    ) {
        (true, true) => println!("Can't be server and client at the same time."),
        (true, false) => {
            // Act as server.
            alice.listen(address.to_string())
        }
        _ => {
            // Act as client.
            // Send alice's inception event to other instance and get its response.
            let response_msg_from_other =
                send_to_other(address.to_string(), alice.log.log.last().unwrap());
            alice.process_response(response_msg_from_other, address.to_string());
            

            alice.log.rotate();
            send_to_other(address.to_string(), alice.log.log.last().unwrap());

            //alice.log.rotate();
            //send_to_other(address.to_string(), alice.log.log.last().unwrap());

            //alice.log.rotate();
            //send_to_other(address.to_string(), alice.log.log.last().unwrap());

        }
    }
}
