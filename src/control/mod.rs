mod http;
mod peer;
mod relay;

use std::env::var;
use std::sync::{Arc, Mutex};

use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;

use crate::control::http::listen;
use crate::control::relay::RelayState;
use crate::worker::{CommandMessage, ServiceMessage};
use crate::{result::*, DEFAULT_CONTROL_SOCKET};

#[derive(Debug)]
pub struct Control {
    service_rcv: broadcast::Receiver<ServiceMessage>,
    listener: JoinHandle<anyhow::Result<()>>,
    state: Arc<Mutex<RelayState>>,
}

impl Control {
    pub fn new(
        service_rcv: broadcast::Receiver<ServiceMessage>,
        command_snd: broadcast::Sender<CommandMessage>,
    ) -> Self {
        let state = Arc::new(Mutex::new(RelayState::default()));

        let listener = tokio::spawn(listen(
            var("TURN_RELAY_CONTROL_ADDR").unwrap_or(DEFAULT_CONTROL_SOCKET.to_string()),
            var("TURN_RELAY_CONTROL_TOKEN").unwrap_or("TurnRelayControlToken".to_string()),
            state.clone(),
            command_snd,
        ));

        Self {
            service_rcv,
            listener,
            state,
        }
    }

    fn handle_service_message(
        &mut self,
        service_message: Result<ServiceMessage, RecvError>,
    ) -> TaskResult {
        match service_message.anyhow().into_recoverable()? {
            ServiceMessage::RelayAllocated(addr) => {
                self.state.lock().unwrap().allocate(addr);
                TaskResult::continued()
            }

            ServiceMessage::RelayConnectionFailed(why) => {
                self.state.lock().unwrap().failure(why);
                TaskResult::continued()
            }

            ServiceMessage::RelayDisconnected => {
                self.state.lock().unwrap().disconnect();
                TaskResult::continued()
            }

            ServiceMessage::RelayRedirected(_) => {
                self.state.lock().unwrap().disconnect();
                TaskResult::continued()
            }

            ServiceMessage::RelayPeerGranted(addr) => {
                if let Some(i) = self.state.lock().unwrap().peer_mut(addr) {
                    i.grant();
                }

                TaskResult::continued()
            }

            ServiceMessage::PeerBound {
                peer_addr,
                local_addr,
            } => {
                if let Some(i) = self.state.lock().unwrap().peer_mut(peer_addr) {
                    i.bind(local_addr);
                }

                TaskResult::continued()
            }

            ServiceMessage::RelayPeerDenied(addr) => {
                if let Some(i) = self.state.lock().unwrap().peer_mut(addr) {
                    i.deny();
                }

                TaskResult::continued()
            }

            ServiceMessage::PeerBindFailed(addr) => {
                if let Some(i) = self.state.lock().unwrap().peer_mut(addr) {
                    i.bind_failed();
                }

                TaskResult::continued()
            }

            ServiceMessage::PeerUnbound(addr) => {
                self.state.lock().unwrap().unbind_peer(addr);
                TaskResult::continued()
            }
        }
    }

    async fn handle_loop(&mut self) -> TaskResult {
        select! {
            service_message = self.service_rcv.recv() => {
                self.handle_service_message(service_message)
            }
            else => {
                if self.listener.is_finished() {
                    println!("Control: finished.");
                    TaskResult::terminate()
                } else {
                    TaskResult::continued()
                }
            }
        }
    }

    pub async fn start(mut self) {
        println!("Control: started");

        loop {
            match self.handle_loop().await {
                Ok(TaskOk::Continue) => {}
                Ok(TaskOk::Terminate) => break,
                Err(TaskErr::RecoverableError(error)) => {
                    eprintln!("Control: Error: {}", error);
                }
                Err(TaskErr::UnrecoverableError(error)) => {
                    eprintln!("Control: Fatal: {}", error);
                    break;
                }
            }
        }

        if let Err(error) = self.listener.await {
            eprintln!("Control: Post-mortem: {}", error);
        }

        println!("Control: stopped");
    }
}
