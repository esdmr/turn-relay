use std::{error, fmt::Debug, future::Future};

#[derive(Debug)]
pub enum TaskOk {
    Continue,
    Terminate,
}

#[derive(Debug)]
pub enum TaskErr {
    RecoverableError(anyhow::Error),
    UnrecoverableError(anyhow::Error),
}

pub type TaskResult = Result<TaskOk, TaskErr>;

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

pub trait ToTaskErr {
    type Item;
    fn into_recoverable(self) -> Result<Self::Item, TaskErr>;
    fn into_unrecoverable(self) -> Result<Self::Item, TaskErr>;
}

impl<T> ToTaskErr for Result<T, anyhow::Error> {
    type Item = T;

    fn into_recoverable(self) -> Result<Self::Item, TaskErr> {
        self.map_err(TaskErr::RecoverableError)
    }

    fn into_unrecoverable(self) -> Result<Self::Item, TaskErr> {
        self.map_err(TaskErr::UnrecoverableError)
    }
}

pub trait TaskResultHelper {
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

impl TaskResultHelper for TaskResult {
    fn continued() -> Self {
        Ok(TaskOk::Continue)
    }

    fn terminate() -> Self {
        Ok(TaskOk::Terminate)
    }
}

pub trait TaskErrHelper {
    type Result;

    #[allow(dead_code)]
    fn catch<R, F>(self, f: F) -> Self::Result
    where
        F: FnOnce(&anyhow::Error) -> Result<R, TaskErr>;

    fn catch_async<RR, R, F>(self, f: F) -> impl Future<Output = Self::Result>
    where
        F: Send + FnOnce(&anyhow::Error) -> RR,
        RR: Send + Future<Output = Result<R, TaskErr>>;
}

impl<T> TaskErrHelper for Result<T, TaskErr>
where
    T: Send,
{
    type Result = Self;

    fn catch<R, F>(self, f: F) -> Self::Result
    where
        F: FnOnce(&anyhow::Error) -> Result<R, TaskErr>,
    {
        if let Err(TaskErr::RecoverableError(e) | TaskErr::UnrecoverableError(e)) = &self {
            f(e)?;
        }

        self
    }

    async fn catch_async<RR, R, F>(self, f: F) -> Self::Result
    where
        F: Send + FnOnce(&anyhow::Error) -> RR,
        RR: Send + Future<Output = Result<R, TaskErr>>,
    {
        if let Err(TaskErr::RecoverableError(e) | TaskErr::UnrecoverableError(e)) = &self {
            f(e).await?;
        }

        self
    }
}

impl<T> TaskErrHelper for Result<T, anyhow::Error>
where
    T: Send,
{
    type Result = Result<T, TaskErr>;

    #[inline]
    fn catch<R, F>(self, f: F) -> Self::Result
    where
        F: FnOnce(&anyhow::Error) -> Result<R, TaskErr>,
    {
        self.into_recoverable().catch(f)
    }

    #[inline]
    async fn catch_async<RR, R, F>(self, f: F) -> Self::Result
    where
        F: Send + FnOnce(&anyhow::Error) -> RR,
        RR: Send + Future<Output = Result<R, TaskErr>>,
    {
        self.into_recoverable().catch_async(f).await
    }
}
