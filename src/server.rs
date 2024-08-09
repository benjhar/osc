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

use std::net::{ToSocketAddrs, UdpSocket};

use crate::{errors::Error, OscMessage};

#[allow(clippy::module_name_repetitions)]
pub struct OscServer {
    listener: UdpSocket,
    buffer: Vec<u8>,
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
        })
    }

    fn handle_request(&self, request: OscMessage) -> Option<OscMessage> {
        todo!()
    }

    pub fn start(&mut self) -> Result<(), Error> {
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
}
