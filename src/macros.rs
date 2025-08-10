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
            $($Variant { $prop, .. })|+ => ::std::mem::replace($prop, $crate::DEFAULT_SOCKET_ADDR),
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
