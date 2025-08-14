mod gui;
mod macros;
mod worker;

use std::net::{IpAddr, SocketAddr};

use crate::macros::addr;

pub const ALL_IP: IpAddr = addr!(0, 0, 0, 0);
pub const LOCAL_IP: IpAddr = addr!(127, 0, 0, 1);
pub const ALL_DYN_SOCKET: SocketAddr = addr!(ALL_IP:0);
pub const LOCAL_DYN_SOCKET: SocketAddr = addr!(LOCAL_IP:0);
pub const DEFAULT_FWD_SOCKET: SocketAddr = addr!(LOCAL_IP:34197);

fn main() -> anyhow::Result<()> {
    gui::run()?;

    Ok(())
}
