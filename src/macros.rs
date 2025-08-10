macro_rules! get_mut {
    ($($Variant:path)|+ => $value:ident.$prop:ident) => {
        match $value {
            $($Variant { $prop, .. })|+ => $prop,
            _ => unreachable!(),
        }
    };
}

pub(crate) use get_mut;

macro_rules! take_value {
    ($($Variant:path)|+ => $self:ident.$prop:ident) => {
        match $self {
            $($Variant { $prop, .. })|+ => ::std::mem::take($prop),
            _ => unreachable!(),
        }
    };
    ($($Variant:path)|+ => $self:ident.$prop:ident: SocketAddr) => {
        match $self {
            $($Variant { $prop, .. })|+ => ::std::mem::replace($prop, $crate::ALL_DYN_SOCKET),
            _ => unreachable!(),
        }
    };
    ($($Variant:path)|+ => $self:ident.$prop:ident ?? $value:expr) => {
        match $self {
            $(Self::$Variant { $prop, .. })|+ => ::std::mem::replace($prop, $value),
            _ => unreachable!(),
        }
    };
}

pub(crate) use take_value;

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
