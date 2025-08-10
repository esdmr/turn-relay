use std::{error, fmt::Debug, future::Future, net::SocketAddr};

use futures::{pending, StreamExt};
use turnclient::{MessageFromTurnServer, TurnClient};

#[derive(Debug, Clone)]
pub enum ServiceMessage {
    RelayAllocated(SocketAddr),
    RelayDisconnected,
    RelayConnectionFailed(String),
    RelayRedirected(SocketAddr),
    RelayPeerGranted(SocketAddr),
    RelayPeerDenied(SocketAddr),
    PeerBound {
        peer_addr: SocketAddr,
        local_addr: SocketAddr,
    },
    PeerBindFailed(SocketAddr),
    PeerUnbound(SocketAddr),
}

#[derive(Debug, Clone)]
pub enum CommandMessage {
    ConnectRelay {
        server: String,
        username: String,
        password: String,
    },
    ConnectPeer {
        peer_addr: SocketAddr,
        local_addr: Option<SocketAddr>,
    },
    ChangeFwdAddr(SocketAddr),
    DisconnectAll,
    DisconnectPeer(SocketAddr),
    TerminateAll,
}

pub type DataMessage = (SocketAddr, Vec<u8>);

#[derive(Debug)]
pub enum WorkerOk {
    Continue,
    Terminate,
}

#[derive(Debug)]
pub enum WorkerErr {
    RecoverableError(anyhow::Error),
    UnrecoverableError(anyhow::Error),
}

pub type WorkerResult = Result<WorkerOk, WorkerErr>;

pub trait ToAnyhowResult {
    type Item;
    fn anyhow(self) -> Result<Self::Item, anyhow::Error>;
}

impl<T, E> ToAnyhowResult for Result<T, E>
where
    E: error::Error + Send + Sync + 'static,
{
    type Item = T;

    fn anyhow(self) -> Result<Self::Item, anyhow::Error> {
        self.map_err(|e| anyhow::Error::new(e))
    }
}

pub trait ToWorkerErr {
    type Item;
    fn into_recoverable(self) -> Result<Self::Item, WorkerErr>;
    fn into_unrecoverable(self) -> Result<Self::Item, WorkerErr>;
}

impl<T> ToWorkerErr for Result<T, anyhow::Error> {
    type Item = T;

    fn into_recoverable(self) -> Result<Self::Item, WorkerErr> {
        self.map_err(WorkerErr::RecoverableError)
    }

    fn into_unrecoverable(self) -> Result<Self::Item, WorkerErr> {
        self.map_err(WorkerErr::UnrecoverableError)
    }
}

pub trait WorkerResultHelper {
    fn continued() -> Self;
    fn terminate() -> Self;

    fn terminate_if(cond: bool) -> Self
    where
        Self: Sized,
    {
        if cond {
            Self::terminate()
        } else {
            Self::continued()
        }
    }
}

impl WorkerResultHelper for WorkerResult {
    fn continued() -> Self {
        Ok(WorkerOk::Continue)
    }

    fn terminate() -> Self {
        Ok(WorkerOk::Terminate)
    }
}

pub trait WorkerErrHelper {
    type Result;
    fn catch<R, F>(self, f: F) -> Self::Result
    where
        F: FnOnce(&anyhow::Error) -> Result<R, WorkerErr>;

    fn catch_async<RR, R, F>(self, f: F) -> impl Future<Output = Self::Result>
    where
        F: Send + FnOnce(&anyhow::Error) -> RR,
        RR: Future<Output = Result<R, WorkerErr>>;
}

impl<T> WorkerErrHelper for Result<T, WorkerErr> {
    type Result = Self;

    fn catch<R, F>(self, f: F) -> Self::Result
    where
        F: FnOnce(&anyhow::Error) -> Result<R, WorkerErr>,
    {
        if let Err(WorkerErr::RecoverableError(e) | WorkerErr::UnrecoverableError(e)) = &self {
            f(e)?;
        }

        self
    }

    async fn catch_async<RR, R, F>(self, f: F) -> Self::Result
    where
        F: Send + FnOnce(&anyhow::Error) -> RR,
        RR: Future<Output = Result<R, WorkerErr>>,
    {
        if let Err(WorkerErr::RecoverableError(e) | WorkerErr::UnrecoverableError(e)) = &self {
            f(e).await?;
        }

        self
    }
}

impl<T> WorkerErrHelper for Result<T, anyhow::Error> {
    type Result = Result<T, WorkerErr>;

    #[inline]
    fn catch<R, F>(self, f: F) -> Self::Result
    where
        F: FnOnce(&anyhow::Error) -> Result<R, WorkerErr>,
    {
        self.into_recoverable().catch(f)
    }

    #[inline]
    async fn catch_async<RR, R, F>(self, f: F) -> Self::Result
    where
        F: Send + FnOnce(&anyhow::Error) -> RR,
        RR: Future<Output = Result<R, WorkerErr>>,
    {
        self.into_recoverable().catch_async(f).await
    }
}

pub struct MaybeTurnClient(pub Option<TurnClient>);

impl MaybeTurnClient {
    pub async fn next(&mut self) -> Option<Result<MessageFromTurnServer, anyhow::Error>> {
        if let Some(client) = &mut self.0 {
            client.next().await
        } else {
            loop {
                pending!();
                eprintln!("Relay: Warning: Client was polled again but it is not connected yet");
            }
        }
    }
}

impl Debug for MaybeTurnClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(_) => write!(f, "MaybeTurnClient(Some)"),
            None => write!(f, "MaybeTurnClient(None)"),
        }
    }
}
