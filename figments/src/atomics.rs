use core::sync::atomic::{AtomicUsize, Ordering};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

#[derive(Default, Debug)]
pub struct AtomicMutex<T> {
    inner: UnsafeCell<T>,
    status: AtomicUsize
}

pub struct AtomicGuard<'a, T> {
    mutex: &'a AtomicMutex<T>
}

impl<T> Deref for AtomicGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.inner.get() }
    }
}

impl<T> DerefMut for AtomicGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.inner.get() }
    }
}

impl<T> Drop for AtomicGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.status.store(0, Ordering::Release);
    }
}

unsafe impl<T: Send> Send for AtomicMutex<T> {}
unsafe impl<T: Send> Sync for AtomicMutex<T> {}

impl<T> AtomicMutex<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner: UnsafeCell::new(inner),
            status: AtomicUsize::new(0)
        }
    }

    pub fn lock(&self) -> Result<AtomicGuard<T>, ()> {
        loop {
            match self.try_lock() {
                Ok(guard) => return Ok(guard),
                _ => continue
            }
        }
    }

    pub fn try_lock(&self) -> Result<AtomicGuard<T>, ()> {
        match self.status.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed) {
            Ok(_) => Ok(AtomicGuard { mutex:  self }),
            Err(_) => Err(())
        }
    }
}
