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
    net::{ToSocketAddrs, UdpSocket},
};

use crate::{errors::Error, Arg, OscMessage};

#[allow(clippy::module_name_repetitions)]
pub struct OscServer {
    listener: UdpSocket,
    buffer: Vec<u8>,
    route_table: HashMap<String, fn(OscMessage) -> Option<OscMessage>>,
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

    fn handle_request(&self, request: OscMessage) -> Option<OscMessage> {
        match self.route_table.get(&request.address) {
            Some(func) => func(request),
            None => None,
        }
    }

    pub fn start(mut self) -> Result<(), Error> {
        loop {
            let (size, sender) = self
                .listener
                .recv_from(&mut self.buffer)
                .map_err(Error::Socket)?;
            let message = OscMessage::parse_bytes(&self.buffer)?;
            if let Some(response) = self.handle_request(message) {
                self.listener.send_to(&response.build()?, sender);
            }
        }

        Ok(())
    }

    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn add_route(
        mut self,
        uri: impl ToString,
        func: fn(OscMessage) -> Option<OscMessage>,
    ) -> Self {
        self.route_table
            .insert(uri.to_string(), func)
            .expect("URI already added to route table");
        self
    }
}
