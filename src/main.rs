use std::{error::Error, str::from_utf8, sync::Arc};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

use clap::App as clapapp;
use clap::Arg;
use keri::{
    event::event_data::EventData, event_message::parse, event_message::SignedEventMessage,
    prefix::Prefix, state::IdentifierState,
};

mod log_state;

#[derive(Clone)]
struct KeriInstance {
    log: log_state::LogState,
    state: IdentifierState,
}

impl KeriInstance {
    fn new(log: log_state::LogState, state: IdentifierState) -> Self {
        KeriInstance {
            log: log,
            state: state,
        }
    }

    fn parse_event(&mut self, event: &str) -> Vec<u8> {
        let mut response: Vec<u8> = vec![];

        // Deserialize signed msg
        let msg = parse::signed_message(event)
            .expect("Can't parse event msg")
            .1;
        let m = msg.clone();

        println!("Process keri event ...");

        // Process message.
        response = match msg.event_message.event.event_data {
            // if it's receipt message, verify it and add to sigs_map.
            EventData::Vrc(_) => {
                println!("Recipt message, verifying ...");
                self.log
                    .add_sig(&self.state.clone(), msg)
                    .expect("Can't verify receipt msg");
                println!(
                    "Got receipt of {:?}-th event",
                    m.clone().event_message.event.sn
                );
                vec![]
            }
            // if it's inception event respond with last establishment message and receipt message.
            EventData::Icp(_) => {
                self.state = self
                    .state
                    .clone()
                    .verify_and_apply(&msg)
                    .expect("Can't verify received message.");
                let receipt = self
                    .log
                    .make_rct(msg.event_message)
                    .expect("Can't make a receipt");

                // Respond with last establishment message and receipt message.
                let last_est = self
                    .log
                    .log
                    .last()
                    .expect("There is no last alice's establishment event.");
                let respond =
                    [last_est.serialize().unwrap(), receipt.serialize().unwrap()].concat();
                println!(
                    "Got inception event from {:?}.",
                    m.event_message.event.prefix.to_str()
                );
                respond
            }
            // if it's rotation event, respond with receipt event.
            EventData::Rot(_) | EventData::Ixn(_) => {
                self.state = self
                    .state
                    .clone()
                    .verify_and_apply(&msg)
                    .expect("Can't verify received message.");
                let receipt = self
                    .log
                    .make_rct(msg.event_message)
                    .expect("Can't make a receipt");

                let respond = receipt.serialize().unwrap();
                println!(
                    "Got rotation event of sn = {:?} from {:?}.",
                    m.event_message.event.sn,
                    m.event_message.event.prefix.to_str()
                );
                respond
            }
            _ => response,
        };
        response
    }
}

//     pub fn process_response(&mut self, response: Vec<SignedEventMessage>, address: String ){
//         // Response is vec of SignedMessage. Handle it one by one. Apply establishment event
//         // to other_state and handle receipt event.
//         for sig_msg in response {
//             match sig_msg.event_message.event.event_data {
//                 // If sig_msg is establishment event, update self.bob_state, and send receipt to bob.
//                 keri::event::event_data::EventData::Icp(_)
//                 | keri::event::event_data::EventData::Rot(_) => {
//                     self.bob_state =
//                         self.bob_state.clone().verify_and_apply(&sig_msg).expect("Can't verify bob's mesage.");
//                     // Send receipt of message sig_msg.
//                     let rcpt = self.log.make_rct(sig_msg.event_message);
//                     send_to_other(address.to_string(), &rcpt.unwrap());
//                 }
//                 // If sig_msg is receipt event, verify it and add to sigs_map.
//                 keri::event::event_data::EventData::Vrc(_) => self
//                     .log
//                     .add_sig(self.bob_state.clone(), sig_msg)
//                     .expect("Can't verify receipt msg."),
//                 _ => {

//                 }
//             }
//         }
//     }
// }

async fn send_event(address: String, last_event: SignedEventMessage) -> Vec<SignedEventMessage> {
    println!("Connecting to TDA on: {}", address);
    let mut stream = TcpStream::connect(address).await.unwrap();

    let event = last_event
        .serialize()
        .expect("Can't deserialize event")
        .clone();
    let result = stream.write(&event).await;
    println!("wrote to stream; success={:?}", result.is_ok());

    // Read the receipt
    let mut buffer = [0; 1024];

    let size = stream.read(&mut buffer[..]).await.expect("Can't get size");
    let receipt_msgs = parse::signed_event_stream(from_utf8(&buffer[..size]).unwrap())
        .unwrap()
        .1;
    receipt_msgs
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments.
    let matches = clapapp::new("get-command-line-args")
        .arg(
            Arg::with_name("host")
                .short('H'.to_string())
                .help("hostname on which we would listen, default: localhost")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .short('P'.to_string())
                .help("port on which we would open TCP connections, default: 49152")
                .takes_value(true),
        )
        .get_matches();

    let host = matches.value_of("host").unwrap_or("localhost");
    let port = matches.value_of("port").unwrap_or("49152");
    let address = [host, ":", port].concat();

    // Create instance of KERI
    let mut keri_instance = Arc::new(Mutex::new(KeriInstance::new(
        log_state::LogState::new().unwrap(),
        IdentifierState::default(),
    )));

    let mut listener = TcpListener::bind(&address).await?;
    println!("TDA Listening on: {}", address);

    loop {
        // Asynchronously wait for an inbound socket.
        let (mut socket, _) = listener.accept().await?;
        let keri = Arc::clone(&keri_instance);
        tokio::spawn(async move {
            let mut buf = [0; 1024];

            // In a loop, read data from the socket
            loop {
                let n = socket
                    .read(&mut buf)
                    .await
                    .expect("failed to read data from socket");

                if n == 0 {
                    return;
                } else {
                    // Read message as utf string
                    let msg = from_utf8(&buf[..n]).unwrap();
                    // Ignore messages shorted then 4 bytes
                    if n > 3 {
                        // Read first 4 characters to see if it match with TDA commands
                        let command = &msg[0..3];
                        match command {
                            "IDS" => {
                                println!("Identifier state");
                                let keri = keri.lock().await;
                                let ids = keri.log.state.clone();
                                println!("SN: {}", ids.sn);
                                let msg = format!("SN: {}\n", ids.sn);

                                socket
                                    .write_all(&msg.as_bytes())
                                    .await
                                    .expect("failed to write data to socket");
                            }
                            "LSE" => {
                                println!("Current KEL:");
                                let keri = keri.lock().await;
                                let kel: Vec<SignedEventMessage> = keri.log.log.clone();
                                for signed_message in &kel {
                                    println!(
                                        "{:?}",
                                        &signed_message.event_message.event.event_data
                                    );
                                    let msg = format!(
                                        "{:?}\n",
                                        &signed_message.event_message.event.event_data
                                    );

                                    socket
                                        .write_all(&msg.as_bytes())
                                        .await
                                        .expect("failed to write data to socket");
                                }
                            }
                            "LSR" => {
                                println!("Current KERL:");
                                let keri = keri.lock().await;
                                let kerl = keri.log.sigs_map.clone();

                                for (key, val) in &kerl {
                                    let msg = format!("{}: {:?}\n", key, val);
                                    socket
                                        .write_all(&msg.as_bytes())
                                        .await
                                        .expect("failed to write data to socket");
                                }
                            }
                            "SEN" => {
                                println!("Received command: {}", msg);
                                // Simple parsing of the command
                                let mut iter = msg.split_whitespace();
                                iter.next();
                                // Get host to where send the message
                                let host = iter.next().unwrap();
                                let port = iter.next().unwrap();
                                let address = [host, ":", port].concat();
                                println!("Send my events to {}", address.clone());
                                let mut keri = keri.lock().await;
                                let last_event = keri.log.log.last().unwrap().clone();

                                // We can get more than one event in response.
                                // Not only receipt events, but also other
                                // types.
                                let response = send_event(address.clone(), last_event).await;
                                println!("Got receipts: {:?}", response);

                                for sig_msg in response {
                                    match sig_msg.event_message.event.event_data {
                                        // If sig_msg is receipt event, verify
                                        // it and add to sigs_map.
                                        EventData::Vrc(_) => {
                                            let validator = keri.clone().state;
                                            println!("Got receipt from {}\n", address.clone());
                                            keri.log
                                                .add_sig(&validator, sig_msg)
                                                .expect("Can't verify receipt msg.");
                                        }
                                        // If sig_msg is event of other type,
                                        // update keri.state and send receipt of
                                        // it to responder.
                                        _ => {
                                            println!("Got event from {}", address.clone());
                                            keri.state = keri
                                                .state
                                                .clone()
                                                .verify_and_apply(&sig_msg)
                                                .expect("Can't verify mesage from response.");
                                            // Send receipt of message sig_msg.
                                            let rcpt = keri
                                                .log
                                                .make_rct(sig_msg.event_message)
                                                .expect("Can't make a receipt");
                                            send_event(address.clone(), rcpt);
                                        }
                                    }
                                }
                            }
                            "ROT" => {
                                println!("Generate rotate event");
                                let mut keri = keri.lock().await;
                                keri.log.rotate();
                            }
                            "IXN" => {
                                let mut iter = msg.split_whitespace();
                                iter.next();
                                // Get payload
                                let payload = iter.next();
                                match payload {
                                    Some(p) => {
                                        let mut keri = keri.lock().await;
                                        keri.log.make_ixn(p);
                                    }
                                    None => {
                                        socket
                                            .write_all(b"Cannot parse the payload\n")
                                            .await
                                            .expect("failed to write data to socket");
                                    }
                                }
                            }
                            // If we do not match any command then probably we are getting keri events
                            _ => {
                                println!("KERI event message. Processing ...");
                                println!("Keri event: {} ", msg);
                                let mut keri = keri.lock().await;
                                let receipt = keri.parse_event(&msg.to_string());
                                println!("Respond with {:?}", String::from_utf8(receipt.clone()));
                                // Send back the receipt

                                socket
                                    .write_all(&receipt)
                                    .await
                                    .expect("failed to write data to socket");
                            }
                        }
                    }
                }
            }
        });
    }
}
