use std::{
    collections::VecDeque,
    io::{ErrorKind, Read, Write},
    net::{TcpStream, ToSocketAddrs, UdpSocket},
    time::{Duration, Instant},
};

use crate::{errors::Error, OscMessage};

pub trait Connection
where
    Self: Sized,
{
    /// Creates a new ``impl Connection``
    ///
    /// # Errors
    /// If creating the new ``impl Connection`` fails, return Err
    fn new<A: ToSocketAddrs, B: ToSocketAddrs>(
        local_address: A,
        remote_address: B,
    ) -> std::io::Result<Self>;
    /// Sends ``buf`` over the ``impl Connection``, returning the size of the data sent.
    ///
    /// # Errors
    /// If sending data fails, return Err
    fn send(&mut self, buf: &[u8]) -> std::io::Result<usize>;
    /// Receives data into ``buf`` over the ``impl Connection``, returning the size of the data
    /// received.
    ///
    /// # Errors
    /// If there is no data to receive, return ``Err(io::Error.kind() == ErrorKind::WouldBlock)``.
    /// If it fails for any other reason, ``Err`` also.
    fn recv(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    /// Sets the read timeout for the ``impl Connection``.
    ///
    /// # Errors
    /// Will return Err if the read timeout could not be set.
    fn set_read_timeout(&self, dur: Option<Duration>) -> std::io::Result<()>;
    /// Sets the ``impl Connection``'s blocking mode.
    ///
    /// # Errors
    /// If the mode cannot be changed, will return an error with kind ``io::ErrorKind::WouldBlock``
    fn set_nonblocking(&self, nonblocking: bool) -> std::io::Result<()>;
    /// Returns a new ``impl Connection`` that references the same underlying network connection.
    ///
    /// # Errors
    /// Failure depends on platform. Some platforms do not implement socket cloning (e.g. WASI/WASM).
    /// Different platforms may generate different errors.
    fn try_clone(&self) -> std::io::Result<Self>;
}

impl Connection for UdpSocket {
    fn new<A: ToSocketAddrs, B: ToSocketAddrs>(
        local_address: A,
        remote_address: B,
    ) -> std::io::Result<Self> {
        let sock = UdpSocket::bind(local_address)?;
        sock.connect(remote_address)?;
        Ok(sock)
    }

    fn send(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        UdpSocket::send(self, buf)
    }

    fn recv(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        UdpSocket::recv(self, buf)
    }

    fn set_read_timeout(&self, dur: Option<Duration>) -> std::io::Result<()> {
        UdpSocket::set_read_timeout(self, dur)
    }

    fn set_nonblocking(&self, nonblocking: bool) -> std::io::Result<()> {
        UdpSocket::set_nonblocking(self, nonblocking)
    }

    fn try_clone(&self) -> std::io::Result<Self> {
        UdpSocket::try_clone(self)
    }
}
impl Connection for TcpStream {
    fn new<A: ToSocketAddrs, B: ToSocketAddrs>(_: A, remote_address: B) -> std::io::Result<Self> {
        TcpStream::connect(remote_address)
    }

    fn send(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write(buf)
    }

    fn recv(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        TcpStream::read(self, buf)
    }

    fn set_read_timeout(&self, dur: Option<Duration>) -> std::io::Result<()> {
        TcpStream::set_read_timeout(self, dur)
    }

    fn set_nonblocking(&self, nonblocking: bool) -> std::io::Result<()> {
        TcpStream::set_nonblocking(self, nonblocking)
    }

    fn try_clone(&self) -> std::io::Result<Self> {
        TcpStream::try_clone(self)
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct OscClient<C: Connection> {
    connection: C,
    message_queue: VecDeque<OscMessage>,
    timeout_secs: f32,
    buffer: Vec<u8>,
}

impl<C: Connection> OscClient<C> {
    /// Creates a new ``OscClient``, listening at ``client_address``, and connected to
    /// ``remote_address``. ``buffer_size`` dictates the maximum size message that the client can
    /// receive (See ``recv`` docs).
    ///
    /// # Errors
    /// If the connection cannot be made, or the read timeout cannot be set, this function will
    /// return an ``Error::Socket``.
    pub fn new<A: ToSocketAddrs, B: ToSocketAddrs>(
        client_address: A,
        remote_address: B,
        buffer_size: usize,
        timeout_secs: Option<f32>,
    ) -> Result<Self, Error> {
        let connection = C::new(client_address, remote_address).map_err(Error::Socket)?;
        connection
            .set_read_timeout(timeout_secs.map(Duration::from_secs_f32))
            .map_err(Error::Socket)?;
        Ok(Self {
            connection,
            message_queue: VecDeque::new(),
            timeout_secs: timeout_secs.unwrap_or(1.0),
            buffer: vec![0; buffer_size],
        })
    }

    /// Sends ``message`` over client's underlying connection.
    ///
    /// # Errors
    /// Will return ``Err`` if ``message.build`` (see relevant docs), or if the connection fails
    /// to send ``message``, will return an ``Error::Socket``
    pub fn send(&mut self, messsage: &OscMessage) -> Result<usize, Error> {
        self.connection
            .send(&messsage.build()?)
            .map_err(Error::Socket)
    }

    /// Sends raw bytes. This function may be useful if your target does not implement standard
    /// OSC, and so would not understand/respond to regular ``send``.
    ///
    /// # Errors
    /// Will return an ``Error::Socket`` if sending the data fails.
    pub fn send_bytes(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        self.connection.send(bytes).map_err(Error::Socket)
    }

    // This returns "Error: Resource temporarily unavailable" if `buf` cannot
    // fit the message
    /// Receives data and parses it into an ``OscMessage``
    ///
    /// # Errors
    /// If no data is ready to be received, or ``self.buffer`` is too small to contain the full
    /// message, this function will return an ``Error::Socket`` containing an error of kind
    /// ``io::ErrorKind::WouldBlock``.
    /// Will also error if ``OscMessage::parse_bytes`` fails. See ``parse_bytes`` docs.
    pub fn recv(&mut self) -> Result<OscMessage, Error> {
        self.connection
            .recv(&mut self.buffer)
            .map_err(Error::Socket)?;
        OscMessage::parse_bytes(&self.buffer)
    }

    fn handle_waiting_errors(
        &mut self,
        res: Result<OscMessage, Error>,
        addr: &impl ToString,
    ) -> Result<Option<OscMessage>, Error> {
        match res {
            Ok(msg) => {
                if msg.address == addr.to_string() {
                    return Ok(Some(msg));
                }

                self.message_queue.push_back(msg);
                Ok(None)
            }
            Err(Error::Socket(e)) => match e.kind() {
                ErrorKind::WouldBlock => Ok(None),
                _ => Err(Error::Socket(e)),
            },
            Err(e) => Err(e),
        }
    }

    /// Wait to receive data meant for ``addr``.
    ///
    /// # Errors
    /// Will return ``Err(Error::Socket(io::Error.kind() == ErrorKind::TimedOut))`` if waiting for
    /// data takes longer than ``self.timeout_secs``
    /// Will also return ``Err(Error::Socket)`` if the call to ``connection.recv`` returns an error
    /// other than ``io::Error::WouldBlock``
    pub fn wait_for(&mut self, addr: &impl ToString) -> Result<OscMessage, Error> {
        for i in 0..self.message_queue.len() {
            if self.message_queue[i].address == addr.to_string() {
                let msg = unsafe { self.message_queue.remove(i).unwrap_unchecked() };
                return Ok(msg);
            }
        }

        let rec = self.recv();
        if let Some(msg) = self.handle_waiting_errors(rec, addr)? {
            return Ok(msg);
        }

        let loop_start = Instant::now();
        loop {
            let rec = self.recv();
            if let Some(msg) = self.handle_waiting_errors(rec, addr)? {
                return Ok(msg);
            }

            let duration = loop_start.elapsed().as_secs_f32();
            if duration >= self.timeout_secs {
                return Err(Error::Socket(std::io::Error::new(
                    ErrorKind::TimedOut,
                    format!("Waiting for data timed out after {duration} seconds"),
                )));
            }
        }
    }

    /// Attempts to clone the ``XAirClient``
    ///
    /// # Errors
    /// Will return ``Err(Error::Socket)`` if the underlying connection failed
    /// to be cloned.
    pub fn try_clone(&self) -> Result<Self, Error> {
        Ok(Self {
            connection: self.connection.try_clone().map_err(Error::Socket)?,
            message_queue: VecDeque::new(),
            timeout_secs: self.timeout_secs,
            buffer: vec![0; self.buffer.len()],
        })
    }
}
