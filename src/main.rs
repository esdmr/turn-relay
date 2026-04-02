mod addr;
mod control;
mod coordinator;
mod gui;
mod result;
mod worker;

use std::{
    env::{args, var},
    net::{IpAddr, SocketAddr},
};

use crate::{addr::addr, result::ToAnyhowResult};

pub(crate) const ALL_IP: IpAddr = addr!(0, 0, 0, 0);
pub(crate) const LOCAL_IP: IpAddr = addr!(127, 0, 0, 1);
pub(crate) const ALL_DYN_SOCKET: SocketAddr = addr!(ALL_IP:0);
pub(crate) const LOCAL_DYN_SOCKET: SocketAddr = addr!(LOCAL_IP:0);
pub(crate) const DEFAULT_FWD_SOCKET: SocketAddr = addr!(LOCAL_IP:34197);
pub(crate) const DEFAULT_CONTROL_SOCKET: SocketAddr = addr!(LOCAL_IP:18576);
pub(crate) const VERSION: &str = "v1.1.0";

fn print_help() {
    println!(
        "\
Usage: turn_relay [-h | --help | -v | --version]

Environment variables:
    TURN_RELAY_HEADLESS=1  Only run the workers. Currently meaningless.

{VERSION} https://github.com/esdmr/turn-relay"
    );
}

fn print_version() {
    println!("{VERSION}");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    for i in args().skip(1) {
        if i.len() >= 1 && i[0..2] == *"--" {
            if i.len() == 2 {
                break;
            }

            if i[3..] == *"help" {
                print_help();
                return Ok(());
            } else if i[3..] == *"version" {
                print_version();
                return Ok(());
            }
        } else if i.len() >= 2 && i[0..1] == *"-" {
            for j in i[1..].chars() {
                match j {
                    'h' => {
                        print_help();
                        return Ok(());
                    }
                    'v' => {
                        print_version();
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        return Err(anyhow::anyhow!("Unknown argument: {i}"));
    }

    let mut coord = coordinator::Coordinator::new();
    let mut gui = None;

    if var("TURN_RELAY_HEADLESS") != Ok("1".to_string()) {
        println!("Running in GUI mode.");
        gui = Some(coord.run_gui());
    } else {
        println!("Running in Headless mode.");
    }

    if var("TURN_RELAY_CONTROL") == Ok("1".to_string()) {
        println!("Running HTTP control.");
        coord.run_control();
    }

    let bg = tokio::spawn(coord.start());
    let mut result: anyhow::Result<()> = Ok(());

    if let Some(i) = gui {
        result = i();
    }

    bg.await.anyhow()??;
    result
}
