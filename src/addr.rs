macro_rules! addr {
    ($a:tt, $b:tt, $c:tt, $d:tt) => {
        ::std::net::IpAddr::V4(::std::net::Ipv4Addr::new($a, $b, $c, $d))
    };
    ($a:tt, $b:tt, $c:tt, $d:tt:$z:tt) => {
        ::std::net::SocketAddr::new(addr!($a:tt.$b:tt.$c:tt.$d:tt), $z)
    };
    ($a:path:$z:tt) => {
        ::std::net::SocketAddr::new($a, $z)
    };
    (($a:expr):$z:tt) => {
        ::std::net::SocketAddr::new($a, $z)
    };
}

pub(crate) use addr;
