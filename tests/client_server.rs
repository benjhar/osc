use std::{net::UdpSocket, thread};

use osc::{client::OscClient, server::OscServer, Arg, OscMessage};

#[test]
fn bind_udp_client() {
    let server = OscServer::new("127.0.0.1:0", 1024).expect("Failed to create server");
    let client = OscClient::<UdpSocket>::new("127.0.0.1:0", server.address(), 1024, Some(1.0));
    assert!(client.is_ok());
}

#[test]
fn handle_client_request() {
    fn ping(_: &OscMessage) -> Option<Vec<Arg>> {
        Some(vec![Arg::Str("pong".to_string())])
    }

    let server = OscServer::new("127.0.0.1:0", 1024)
        .expect("Failed to create server")
        .add_route("/ping", ping);

    let mut client = OscClient::<UdpSocket>::new("127.0.0.1:0", server.address(), 1024, Some(1.0))
        .expect("Could not create client");

    thread::spawn(move || {
        server.start().expect("Server crashed");
    });

    let message = OscMessage::new("/ping", vec![]);
    client.send(&message).expect("Could not send message");
    let response = client.recv().expect("Received no message");
    assert!(response.args[0] == Arg::Str("pong".to_string()));
}
