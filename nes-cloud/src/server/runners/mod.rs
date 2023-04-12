use std::sync::mpsc::Sender;

use crate::server::Client;
use crate::server::Event;
use crate::AppSettings;

pub mod docker;
mod fcntl;
pub mod process;

pub trait InstanceRunner {
  fn run(
    &mut self,
    client: Client,
    tx: Sender<Event>,
    settings: &AppSettings,
    current_players: usize,
  ) -> Result<(), Box<dyn std::error::Error>>;
}
