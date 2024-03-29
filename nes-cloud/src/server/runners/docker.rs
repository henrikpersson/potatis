use super::InstanceRunner;
use crate::server::Client;
use crate::AppSettings;

struct DockerInstanceRunner;

impl InstanceRunner for DockerInstanceRunner {
  fn run(
    &mut self,
    _client: Client,
    _tx: std::sync::mpsc::Sender<crate::server::Event>,
    _settings: &AppSettings,
    _current_players: usize,
  ) -> Result<(), Box<dyn std::error::Error>> {
    todo!()
  }
}
