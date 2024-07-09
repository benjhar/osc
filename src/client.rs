use std::{
    collections::VecDeque,
    io::{Error, ErrorKind, Read, Write},
    net::{TcpStream, ToSocketAddrs, UdpSocket},
    time::{Duration, Instant},
};

use crate::OscMessage;

pub trait Connection
where
    Self: Sized,
{
    fn new<A: ToSocketAddrs, B: ToSocketAddrs>(
        local_address: A,
        remote_address: B,
    ) -> Result<Self, Error>;
    fn send(&mut self, buf: &[u8]) -> Result<usize, Error>;
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Error>;
    fn set_read_timeout(&self, dur: Option<Duration>) -> Result<(), Error>;
    fn set_nonblocking(&self, nonblocking: bool) -> Result<(), Error>;
}

impl Connection for UdpSocket {
    fn new<A: ToSocketAddrs, B: ToSocketAddrs>(
        local_address: A,
        remote_address: B,
    ) -> Result<Self, Error> {
        let sock = UdpSocket::bind(local_address)?;
        sock.connect(remote_address)?;
        Ok(sock)
    }

    fn send(&mut self, buf: &[u8]) -> Result<usize, Error> {
        UdpSocket::send(self, buf)
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        UdpSocket::recv(self, buf)
    }

    fn set_read_timeout(&self, dur: Option<Duration>) -> Result<(), Error> {
        UdpSocket::set_read_timeout(self, dur)
    }

    fn set_nonblocking(&self, nonblocking: bool) -> Result<(), Error> {
        UdpSocket::set_nonblocking(self, nonblocking)
    }
}
impl Connection for TcpStream {
    fn new<A: ToSocketAddrs, B: ToSocketAddrs>(_: A, remote_address: B) -> Result<Self, Error> {
        TcpStream::connect(remote_address)
    }

    fn send(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.write(buf)
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        TcpStream::read(self, buf)
    }

    fn set_read_timeout(&self, dur: Option<Duration>) -> Result<(), Error> {
        TcpStream::set_read_timeout(self, dur)
    }

    fn set_nonblocking(&self, nonblocking: bool) -> Result<(), Error> {
        TcpStream::set_nonblocking(self, nonblocking)
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
        let connection = C::new(client_address, remote_address)?;
        connection.set_read_timeout(timeout_secs.map(Duration::from_secs_f32))?;
        Ok(Self {
            connection,
            message_queue: VecDeque::new(),
            timeout_secs: timeout_secs.unwrap_or(1.0),
            buffer: vec![0; buffer_size],
        })
    }

    pub fn send(&mut self, messsage: OscMessage) -> Result<usize, Error> {
        self.connection.send(&messsage.build())
    }

    pub fn send_bytes(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        self.connection.send(bytes)
    }

    // This returns "Error: Resource temporarily unavailable" if `buf` cannot
    // fit the message
    pub fn recv(&mut self) -> Result<OscMessage, Error> {
        self.connection.recv(&mut self.buffer)?;
        OscMessage::parse_bytes(&self.buffer)
    }

    pub fn wait_for(&mut self, addr: impl ToString) -> Result<OscMessage, Error> {
        for i in 0..self.message_queue.len() {
            if self.message_queue[i].address == addr.to_string() {
                let msg = self.message_queue.remove(i).unwrap();
                return Ok(msg);
            }
        }

        match self.recv() {
            Ok(msg) => {
                if msg.address == addr.to_string() {
                    return Ok(msg);
                }

                self.message_queue.push_back(msg);
            }
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => {}
                _ => return Err(e),
            },
        }

        let loop_start = Instant::now();
        loop {
            match self.recv() {
                Ok(msg) => {
                    if msg.address == addr.to_string() {
                        return Ok(msg);
                    }

                    self.message_queue.push_back(msg);
                }
                Err(e) => match e.kind() {
                    ErrorKind::WouldBlock => {}
                    _ => return Err(e),
                },
            }
            let duration = (Instant::now() - loop_start).as_secs_f32();
            if duration >= self.timeout_secs {
                return Err(Error::new(
                    ErrorKind::TimedOut,
                    format!("Waiting for data timed out after {duration} seconds"),
                ));
            }
        }
    }
}
