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
    fn new<A: ToSocketAddrs, B: ToSocketAddrs>(
        local_address: A,
        remote_address: B,
    ) -> std::io::Result<Self>;
    fn send(&mut self, buf: &[u8]) -> std::io::Result<usize>;
    fn recv(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    fn set_read_timeout(&self, dur: Option<Duration>) -> std::io::Result<()>;
    fn set_nonblocking(&self, nonblocking: bool) -> std::io::Result<()>;
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

#[derive(Clone)]
pub struct OscClient<C: Connection> {
    connection: C,
    message_queue: VecDeque<OscMessage>,
    timeout_secs: f32,
    buffer: Vec<u8>,
}

impl<C: Connection> OscClient<C> {
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

    pub fn send(&mut self, messsage: OscMessage) -> Result<usize, Error> {
        self.connection
            .send(&messsage.build())
            .map_err(Error::Socket)
    }

    pub fn send_bytes(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        self.connection.send(bytes).map_err(Error::Socket)
    }

    // This returns "Error: Resource temporarily unavailable" if `buf` cannot
    // fit the message
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

    pub fn wait_for(&mut self, addr: impl ToString) -> Result<OscMessage, Error> {
        for i in 0..self.message_queue.len() {
            if self.message_queue[i].address == addr.to_string() {
                let msg = self.message_queue.remove(i).unwrap();
                return Ok(msg);
            }
        }

        let rec = self.recv();
        if let Some(msg) = self.handle_waiting_errors(rec, &addr)? {
            return Ok(msg);
        }

        let loop_start = Instant::now();
        loop {
            let rec = self.recv();
            if let Some(msg) = self.handle_waiting_errors(rec, &addr)? {
                return Ok(msg);
            }

            let duration = (Instant::now() - loop_start).as_secs_f32();
            if duration >= self.timeout_secs {
                return Err(Error::Socket(std::io::Error::new(
                    ErrorKind::TimedOut,
                    format!("Waiting for data timed out after {duration} seconds"),
                )));
            }
        }
    }

    pub fn try_clone(&self) -> Result<Self, Error> {
        Ok(Self {
            connection: self.connection.try_clone().map_err(Error::Socket)?,
            message_queue: VecDeque::new(),
            timeout_secs: self.timeout_secs,
            buffer: vec![0; self.buffer.len()],
        })
    }
}
