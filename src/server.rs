// Forbidden characters in OSC addresses:
// space
// #
// *
// ,
// /
// ?
// [
// ]
// {
// }

use std::{
    collections::HashMap,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
};

use crate::{errors::Error, Arg, OscMessage};

#[allow(clippy::module_name_repetitions)]
pub struct OscServer {
    listener: UdpSocket,
    buffer: Vec<u8>,
    route_table: HashMap<String, fn(&OscMessage) -> Option<Vec<Arg>>>,
}

impl OscServer {
    /// Creates a new ``OscServer`` listening on ``bind_addr``
    ///
    /// # Errors
    /// Will return an ``Error::Socket`` if a ``Listener`` cannot be bound to ``bind_addr``
    pub fn new<A: ToSocketAddrs>(bind_addr: A, capacity: usize) -> Result<Self, Error> {
        Ok(OscServer {
            listener: UdpSocket::bind(bind_addr).map_err(Error::Socket)?,
            buffer: Vec::with_capacity(capacity),
            route_table: HashMap::new(),
        })
    }

    #[must_use]
    pub fn address(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .expect("Unable to access local addr.")
    }

    fn handle_request(&self, request: &OscMessage) -> Option<Vec<Arg>> {
        match self.route_table.get(&request.address) {
            Some(func) => func(request),
            None => None,
        }
    }

    pub fn start(mut self) -> Result<(), Error> {
        println!("Server starting on {}", self.address());
        loop {
            println!("waiting");
            if let Ok((_, sender)) = self.listener.recv_from(&mut self.buffer) {
                println!("Message received from {sender}: {:?}", self.buffer);
                if let Ok(mut message) = OscMessage::parse_bytes(&self.buffer) {
                    println!(
                        "Destined for: {}, carrying: {:?}",
                        message.address, message.args
                    );
                    if let Some(response) = self.handle_request(&message) {
                        println!("Responding: {response:?}");
                        message.args = response;
                        let _ = self.listener.send_to(&message.build()?, sender);
                    }
                }
            }
        }
    }

    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn add_route(
        mut self,
        uri: impl ToString,
        func: fn(&OscMessage) -> Option<Vec<Arg>>,
    ) -> Self {
        assert!(
            self.route_table.insert(uri.to_string(), func).is_none(),
            "URI already added to route table"
        );
        self
    }
}
