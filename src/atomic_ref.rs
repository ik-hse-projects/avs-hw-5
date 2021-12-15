use std::{
    fmt::Debug,
    marker::PhantomData,
    sync::atomic::{AtomicPtr, Ordering},
};

pub struct AtomicRef<'a, T> {
    ptr: AtomicPtr<T>,
    _phantom: PhantomData<&'a T>,
}

impl<'a, T> AtomicRef<'a, T> {
    pub fn new(value: Option<&'a T>) -> Self {
        let ptr = match value {
            Some(val) => val as *const T as *mut T,
            None => std::ptr::null_mut(),
        };
        Self {
            ptr: AtomicPtr::new(ptr),
            _phantom: Default::default(),
        }
    }

    pub fn load(&self, order: Ordering) -> Option<&'a T> {
        let ptr = self.ptr.load(order) as *const T;
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { &*ptr })
        }
    }

    pub fn store(&self, value: Option<&'a T>, order: Ordering) {
        let ptr = value.map(|x| x as *const T).unwrap_or(std::ptr::null());
        self.ptr.store(ptr as *mut T, order)
    }

    pub fn swap(&self, value: Option<&'a T>, order: Ordering) -> Option<&'a T> {
        let ptr = value.map(|x| x as *const T).unwrap_or(std::ptr::null());
        let res = self.ptr.swap(ptr as *mut T, order);
        if res.is_null() {
            None
        } else {
            Some(unsafe { &*res })
        }
    }
}

impl<'a, T> Debug for AtomicRef<'a, T>
where
    Option<&'a T>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AtomicRef")
            .field(&self.load(Ordering::Relaxed))
            .finish()
    }
}
