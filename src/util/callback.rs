use std::{future::Future, sync::Arc};

use futures::{future::BoxFuture, FutureExt};

pub enum B2Callback<T: Sync + Send + 'static> {
    Fn(Box<dyn Fn(T) + Send + Sync>),
    AsyncFn(Box<dyn Fn(T) -> BoxFuture<'static, ()> + Send + Sync>),
}

impl<T: Sync + Send + 'static> B2Callback<T> {
    /// Construct middleware from function
    pub fn from_fn<F>(fun: F) -> Self
    where
        F: Fn(T) + Send + Sync + 'static,
    {
        B2Callback::Fn(Box::new(fun))
    }

    /// Construct middleware from async function
    pub fn from_async_fn<F, R>(fun: F) -> Self
    where
        F: Fn(T) -> R + Send + Sync + 'static,
        R: Future<Output = ()> + Send + 'static,
    {
        let fun = Arc::new(fun);
        B2Callback::AsyncFn(Box::new(move |bytes| {
            let fun = fun.clone();
            async move {
                let fun = fun.clone();
                fun(bytes).await;
            }
            .boxed()
        }))
    }
}
