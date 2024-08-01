use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs, UdpSocket},
    time::Duration,
};

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
