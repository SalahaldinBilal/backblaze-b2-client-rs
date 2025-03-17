use std::{cell::UnsafeCell, ops::Deref, sync::Arc};

use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};

#[derive(Debug)]
/// An Arc that allows editing the Arc inner value with a lock
pub(crate) struct WriteLockArc<T> {
    arc: Arc<UnsafeCell<T>>,
    write_guard: Arc<Mutex<()>>,
}

unsafe impl<T> Send for WriteLockArc<T> {}
unsafe impl<T> Sync for WriteLockArc<T> {}

impl<T> WriteLockArc<T> {
    pub fn new(data: T) -> Self {
        Self {
            arc: Arc::new(UnsafeCell::new(data)),
            write_guard: Arc::new(Mutex::new(())),
        }
    }

    /// Get lock to inner value for writing (doesn't lock reads)
    pub async fn lock_write(&self) -> MappedMutexGuard<'_, T> {
        let lock = self.write_guard.lock().await;
        MutexGuard::map(lock, |_| unsafe {
            self.arc.get().as_mut().expect("Valid pointer")
        })
    }

    /// Changes the inner value
    pub async fn set(&self, new_value: T) {
        let mut guard = self.lock_write().await;

        *guard = new_value;
    }
}

impl<T> Deref for WriteLockArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.arc.get().as_ref().expect("Valid pointer") }
    }
}

impl<T> Clone for WriteLockArc<T> {
    fn clone(&self) -> Self {
        Self {
            arc: self.arc.clone(),
            write_guard: self.write_guard.clone(),
        }
    }
}
