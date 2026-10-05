use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug)]
/// A shared value behind a lock. Guards are synchronous, so never hold one across an `.await`.
pub(crate) struct WriteLockArc<T>(Arc<RwLock<T>>);

impl<T> WriteLockArc<T> {
    pub fn new(data: T) -> Self {
        Self(Arc::new(RwLock::new(data)))
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.0.read().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn lock_write(&self) -> RwLockWriteGuard<'_, T> {
        self.0.write().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set(&self, new_value: T) {
        *self.lock_write() = new_value;
    }
}

impl<T: Clone> WriteLockArc<T> {
    pub fn get(&self) -> T {
        self.read().clone()
    }
}

impl<T> Clone for WriteLockArc<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
