use std::{error::Error, net::{TcpListener, TcpStream, SocketAddr}, io::Write, time::Duration};
use libcloud::{resources::{Resources, StrId}, ServerMode};
use std::sync::mpsc::Sender;
use log::{info, error, warn};

use crate::{AppSettings, runners::{process::ProcessInstanceRunner, InstanceRunner}};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientId(SocketAddr);

#[derive(Debug)]
pub struct Client {
  pub id: ClientId,
  pub socket: TcpStream,
  pub mode: ServerMode,
}

#[derive(Debug)]
pub enum Event {
  Error(String),
  Connect(Client),
  Disconnect(ClientId),
  Blocked(Client, Vec<u8>)
}

pub struct Server {
  res: Resources,
  settings: AppSettings,
  connected: Vec<ClientId>,
  crd_timeout: Duration,
}

impl Server {
  pub fn new(res: Resources, settings: AppSettings) -> Self {
    Self { 
      res,
      connected: Vec::with_capacity(settings.max_concurrent),
      crd_timeout: Duration::from_millis(settings.client_read_timeout),
      settings,
    }
  }

  pub fn serve(mut self) -> Result<(), Box<dyn Error>> {
    let servers = [
      (ServerMode::User, self.settings.user_port),
      (ServerMode::Color, self.settings.color_port),
      (ServerMode::Ascii, self.settings.ascii_port),
      (ServerMode::Sixel, self.settings.sixel_port),
    ];

    let (tx, rx) = std::sync::mpsc::channel();
    for (mode, port) in servers.into_iter() {
      let host = format!("{}:{}", self.settings.host, port);
      info!("{:?} listening on {}", mode, host);
      let server = TcpListener::bind(host)?;
      Self::start_accepting(server, tx.clone(), mode);
    };
    
    // TODO: Inject?
    let mut runner = ProcessInstanceRunner::new(&self.settings.instance_bin);

    // Main thread
    while let Ok(ev) = rx.recv() {
      match ev {
        Event::Connect(client) => self.client_connected(&mut runner, client, tx.clone()),
        Event::Disconnect(client_id) => self.client_disconnected(client_id),
        Event::Blocked(mut client, msg) => { _ = client.socket.write_all(&msg) },
        Event::Error(e) => error!("Error: {}", e),
      }
    }

    warn!("Server died.");
    Ok(())
  }

  fn start_accepting(srv_socket: TcpListener, tx: Sender<Event>, mode: ServerMode) {
    std::thread::spawn(move || {
      loop {
        match srv_socket.accept() {
          Ok((socket, addr)) => {
            let client = Client { id: ClientId(addr), socket, mode };
            tx.send(Event::Connect(client))
          }
          Err(e) => tx.send(Event::Error(e.to_string())),
        }.unwrap(); // Err == main thread died.
      }
    });
  }

  fn client_disconnected(&mut self, client_id: ClientId) {
    self.connected.retain(|c| c.0.ip() != client_id.0.ip());
    error!("Client disconnected: {:?} ({} connected)", client_id, self.connected.len());
  }

  fn client_connected(&mut self, runner: &mut ProcessInstanceRunner, client: Client, tx: Sender<Event>) {
    info!("Client soft-connect: {:?} ({} connected)", client.id, self.connected.len());

    // Block, with events
    if let Some(msg) = self.block_client(&client.id) {
      warn!("{:?} blocked: {}", client.id, String::from_utf8(msg.to_vec()).unwrap());
      tx.send(Event::Blocked(client, msg.to_vec())).unwrap();
      return;
    }

    if let Err(e) = client.socket.set_read_timeout(Some(self.crd_timeout)) {
      warn!("failed to set client timeout: {:?} {}", client.id, e);
      return;
    }

    let client_id = client.id;
    if let Err(e) = runner.run(client, tx, &self.settings, self.connected.len()) {
      // TODO: Event?
      warn!("Runner failed to start: {}", e);
    } 
    else {
      info!("Client connected! {:?}", client_id);
      self.connected.push(client_id)
    }
  }

  fn block_client(&self, client_id: &ClientId) -> Option<&[u8]> {
    if self.connected.len() >= self.settings.max_concurrent {
      return Some(&self.res[StrId::TooManyPlayers])
    }
    if self.settings.block_dup && 
        self.connected.iter().any(|&c| c.0.ip() == client_id.0.ip()) {
      return Some(&self.res[StrId::AlreadyConnected])
    }
    None
  }
}