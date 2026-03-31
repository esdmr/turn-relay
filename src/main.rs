mod gui;
mod macros;
mod worker;

use std::{
    env::{args, var},
    net::{IpAddr, SocketAddr},
};

use crate::macros::addr;

pub const ALL_IP: IpAddr = addr!(0, 0, 0, 0);
pub const LOCAL_IP: IpAddr = addr!(127, 0, 0, 1);
pub const ALL_DYN_SOCKET: SocketAddr = addr!(ALL_IP:0);
pub const LOCAL_DYN_SOCKET: SocketAddr = addr!(LOCAL_IP:0);
pub const DEFAULT_FWD_SOCKET: SocketAddr = addr!(LOCAL_IP:34197);
pub const VERSION: &str = "v1.1.0";

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

fn main() -> anyhow::Result<()> {
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

    if var("TURN_RELAY_HEADLESS") == Ok("1".to_string()) {
        println!("Running in headless mode.");
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                worker::run_headless().await;
            });
    } else {
        println!("Running in GUI mode.");
        gui::run()?;
    }

    Ok(())
}
