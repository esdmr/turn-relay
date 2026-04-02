use rocket::{get, routes, Build, Config, Rocket, State};
use tokio::sync::broadcast;

use std::{
    net::ToSocketAddrs,
    sync::{Arc, Mutex},
};

use crate::{control::relay::RelayState, worker::CommandMessage, DEFAULT_CONTROL_SOCKET};

struct Token(String);

struct WorkerState(Arc<Mutex<RelayState>>);

struct CommandSnd(broadcast::Sender<CommandMessage>);

#[get("/")]
fn get_relay(state: &State<WorkerState>) -> String {
    serde_json::to_string(&state.0.lock().unwrap().clone()).unwrap()
}

#[get("/relay_addr")]
fn get_relay_addr(state: &State<WorkerState>) -> String {
    serde_json::to_string(&state.0.lock().unwrap().relay_addr()).unwrap()
}

#[get("/fwd_addr")]
fn get_fwd_addr(state: &State<WorkerState>) -> String {
    serde_json::to_string(&state.0.lock().unwrap().fwd_addr()).unwrap()
}

#[get("/peers")]
fn get_peers(state: &State<WorkerState>) -> String {
    serde_json::to_string(&state.0.lock().unwrap().peers()).unwrap()
}

#[get("/peers/<peer>")]
fn get_peer(peer: String, state: &State<WorkerState>) -> String {
    let addr = peer.parse();

    if let Ok(addr) = addr {
        serde_json::to_string(&state.0.lock().unwrap().peer(addr)).unwrap()
    } else {
        serde_json::to_string(&serde_json::Value::Null).unwrap()
    }
}

#[get("/peers/<peer>/local_addr")]
fn get_peer_local_addr(peer: String, state: &State<WorkerState>) -> String {
    let addr = peer.parse();

    if let Ok(addr) = addr {
        serde_json::to_string(&state.0.lock().unwrap().peer(addr).map(|i| i.local_addr())).unwrap()
    } else {
        serde_json::to_string(&serde_json::Value::Null).unwrap()
    }
}

fn rocket() -> Rocket<Build> {
    rocket::build().mount(
        "/",
        routes![
            get_relay,
            get_relay_addr,
            get_fwd_addr,
            get_peers,
            get_peer,
            get_peer_local_addr
        ],
    )
}

pub async fn listen(
    addrs: impl ToSocketAddrs,
    token: String,
    state: Arc<Mutex<RelayState>>,
    command_snd: broadcast::Sender<CommandMessage>,
) -> anyhow::Result<()> {
    let mut addr = DEFAULT_CONTROL_SOCKET;

    for (idx, new_addr) in addrs.to_socket_addrs()?.enumerate() {
        assert_eq!(idx, 0, "Cannot bind control to multiple addresses");
        addr = new_addr;
    }

    let config = Config {
        address: addr.ip(),
        port: addr.port(),
        profile: "release".into(),
        ..Config::debug_default()
    };

    rocket()
        .configure(config)
        .manage(Token(token))
        .manage(WorkerState(state))
        .manage(CommandSnd(command_snd))
        .launch()
        .await?;

    Ok(())
}
