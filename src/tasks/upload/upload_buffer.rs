use bytes::Bytes;

#[derive(Debug, Clone)]
pub(super) struct UploadBuffer(Bytes);

impl UploadBuffer {
    pub fn new<B>(bytes: B) -> Self
    where
        B: Into<Bytes>,
    {
        Self(bytes.into())
    }

    pub fn chunks(&self, chunk_size: usize) -> UploadBufferChunks {
        UploadBufferChunks::new(self.0.clone(), chunk_size)
    }
}

#[derive(Debug, Clone)]
pub(super) struct UploadBufferChunks {
    data: Bytes,
    chunk_size: usize,
    offset: usize,
}

unsafe impl Send for UploadBufferChunks {}
unsafe impl Sync for UploadBufferChunks {}

impl UploadBufferChunks {
    pub fn new(data: Bytes, chunk_size: usize) -> Self {
        Self {
            data,
            chunk_size,
            offset: 0,
        }
    }
}

impl Iterator for UploadBufferChunks {
    type Item = Bytes;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let start = self.offset;

        let end = self.offset + self.chunk_size;
        let end = if end >= self.data.len() {
            self.data.len()
        } else {
            end
        };

        self.offset = end;
        Some(self.data.slice(start..end))
    }
}
