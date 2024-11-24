use std::net::UdpSocket;

use osc::{client::OscClient, OscMessage};

fn main() {
    // Create client, connected to our server
    let mut client = OscClient::<UdpSocket>::new("127.0.0.1:0", "0.0.0.0:47336", 1024, Some(1.0))
        .expect("Could not create client");

    // Create a new message to send to the server.
    let message = OscMessage::new("/ping", vec![]);

    // Send the message to the server
    client.send(&message).expect("Could not send message");

    // Get the server's response
    let response = client.recv().expect("Received no message");

    // Pong!
    println!("{:?}", response.args[0]);
}
