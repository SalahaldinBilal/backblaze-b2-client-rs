use std::pin::Pin;

use bytes::Bytes;
use futures::StreamExt;
use futures_core::Stream;

use crate::error::B2Error;

use super::B2Callback;

/// A file stream for the B2File, you're most likely gonna only use it as the following:
///
/// ```rs
/// let mut response = client
///        .download_file_by_id(B2DownloadFileByIdQueryParameters::builder().file_id("...".into()).build())
///        .await
///        .unwrap();
///
/// let data = response.file.read_all().await;
/// ```
pub struct B2FileStream {
    stream: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>>>>,
    size: usize,
    middlewares: Vec<B2Callback<Bytes>>,
}

impl B2FileStream {
    pub fn new<S>(stream: S, size: usize) -> Self
    where
        S: Stream<Item = Result<Bytes, reqwest::Error>> + 'static,
    {
        Self {
            stream: Box::pin(stream),
            size,
            middlewares: vec![],
        }
    }

    /// Reads the entire file at once, consuming self in the process.
    pub async fn read_all(mut self) -> Result<Bytes, B2Error> {
        let mut buffer: Vec<u8> = Vec::with_capacity(self.size);

        loop {
            match self.stream.next().await {
                Some(value) => {
                    let value = value.map_err(|err| B2Error::RequestSendError(err))?;

                    for middleware in &mut self.middlewares {
                        match middleware {
                            B2Callback::Fn(fun) => fun(value.clone()),
                            B2Callback::AsyncFn(fun) => fun(value.clone()).await,
                        }
                    }

                    buffer.extend_from_slice(value.as_ref());
                }
                None => break,
            }
        }

        Ok(Bytes::from(buffer))
    }

    /// Consumes self, then returns the underlying stream and file size
    pub fn into_stream(
        self,
    ) -> (
        usize,
        Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>>>>,
    ) {
        (self.size, self.stream)
    }

    /// Adds a middleware to the list to run, returns mutable reference to self.
    pub fn add_middleware(&mut self, middleware: B2Callback<Bytes>) -> &mut Self {
        self.middlewares.push(middleware);

        self
    }
}
