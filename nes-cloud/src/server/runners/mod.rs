use std::{sync::mpsc::Sender};
use crate::{server::{Event, Client}, AppSettings};

mod fcntl;
pub mod process;
pub mod docker;

pub trait InstanceRunner {
  fn run(
    &mut self, 
    client: Client,
    tx: Sender<Event>,
    settings: &AppSettings,
    current_players: usize,
  ) -> Result<(), Box<dyn std::error::Error>>;
}
