use std::{path::PathBuf, sync::mpsc::Sender, error::Error, os::fd::AsRawFd, process::Command};

use log::info;

use crate::{server::{Event, Client}, runners::fcntl, AppSettings};

use super::InstanceRunner;

pub struct ProcessInstanceRunner {
  child_binary_path: PathBuf
}

impl ProcessInstanceRunner {
  pub fn new(path: &str) -> Self {
    let child_binary_path = PathBuf::from(path);
    if !child_binary_path.exists() {
      // Fail fast, not on first connect.
      panic!("instance binary does not exist: {:?}", child_binary_path)
    }
    info!("Using binary: {:?}", child_binary_path);
    Self { child_binary_path }
  }
}

impl InstanceRunner for ProcessInstanceRunner {
  fn run(
    &mut self,
    client: Client, 
    tx: Sender<Event>, 
    settings: &AppSettings,
    current_players: usize,
  ) -> Result<(), Box<dyn Error>> {
    fcntl::unset_fd_cloexec(&client.socket);

    let socket_fd = client.socket.as_raw_fd();
    let mut child = Command::new(&self.child_binary_path)
      .arg(format!("{:?}_{}", client.id, socket_fd)) // for ps
      .env("FD", socket_fd.to_string())
      .env("MODE", client.mode.to_string())
      .env("LOG_TO_FILE", settings.log_to_file.to_string())
      .env("PLAYERS", current_players.to_string())
      .spawn()?;

    info!("Spawned instance for fd: {}, pid: {}", socket_fd, child.id());

    std::thread::spawn(move || {
      let code = child.wait();
      info!("Instance {} exited with status {:?}", socket_fd, code);
      tx.send(Event::Disconnect(client.id)).unwrap(); // Err = main thread died
    });

    Ok(())
  }
}