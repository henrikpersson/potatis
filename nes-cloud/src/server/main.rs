use std::{error::Error};

use libcloud::{logging, resources::Resources};
use server::Server;
use structopt::StructOpt;

mod server;
mod runners;

#[derive(StructOpt, Debug)]
pub struct AppSettings {
  #[structopt(long, default_value = "target/release/nes-cloud-instance")]
  pub instance_bin: String,
  #[structopt(long)]
  pub log_to_file: bool,
  #[structopt(short, long)]
  pub block_dup: bool,
  #[structopt(short, long, default_value = "5")]
  pub max_concurrent: usize,
  #[structopt(short = "t", long, default_value = "60000")]
  pub client_read_timeout: u64,
  #[structopt(short, long, default_value = "./resources.yaml")]
  pub resources: String,
  #[structopt(short, long, default_value = "0.0.0.0")]
  pub host: String,
  #[structopt(short, long, default_value = "4444")]
  pub user_port: u16,
  #[structopt(short, long, default_value = "5555")]
  pub color_port: u16,
  #[structopt(short, long, default_value = "6666")]
  pub sixel_port: u16,
  #[structopt(short, long, default_value = "7777")]
  pub ascii_port: u16,
}

fn main() -> Result<(), Box<dyn Error>> {
  let settings: AppSettings = AppSettings::from_args();
  logging::init(settings.log_to_file)?;

  let resources = Resources::load(&settings.resources);
  Server::new(resources, settings).serve()?;

  Ok(())
}