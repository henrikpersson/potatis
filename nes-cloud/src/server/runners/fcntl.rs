use std::{net::TcpStream, os::fd::AsRawFd};

const F_GETFD: i32 = 1;
const F_SETFD: i32 = 2;
const FD_CLOEXEC: i32 = 1;

extern "C" {
  fn fcntl(fd: i32, cmd: i32, ...) -> i32;
}

pub fn unset_fd_cloexec(s: &TcpStream) {
  // SAFETY: Nope!
  unsafe {
    let fd = s.as_raw_fd();
    let mut flags = fcntl(fd, F_GETFD);
    flags &= !FD_CLOEXEC;
    fcntl(fd, F_SETFD, flags);
  }
}