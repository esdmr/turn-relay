mod gui;
mod macros;
mod worker;

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

pub const DEFAULT_SOCKET_ADDR: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 0));

pub const DEFAULT_FWD_SOCKET_ADDR: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 34197));

fn main() -> anyhow::Result<()> {
    gui::run_gui()?;

    Ok(())
}
