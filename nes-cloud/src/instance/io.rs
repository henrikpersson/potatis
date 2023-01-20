use std::{io::{Write, Read, Cursor}, net::TcpStream, time::Duration};

pub enum CloudStream { 
  Offline,
  Online(TcpStream),
}

impl Clone for CloudStream {
  fn clone(&self) -> Self {
    match self {
      Self::Offline => Self::Offline,
      Self::Online(socket) => Self::Online(socket.try_clone().expect("failed to clone socket")),
    }
  }
}

impl Write for CloudStream {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    match self {
      CloudStream::Offline => std::io::stdout().write(buf),
      CloudStream::Online(socket) => socket.write(buf),
    }
  }

  fn flush(&mut self) -> std::io::Result<()> {
    match self {
      CloudStream::Offline => std::io::stdout().flush(),
      CloudStream::Online(socket) => socket.flush(),
    }
  }
}

impl Read for CloudStream {
  fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    match self {
      CloudStream::Offline => std::io::stdin().read(buf),
      CloudStream::Online(socket) => socket.read(buf),
    }
  }
}