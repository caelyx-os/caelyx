use core::{
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug)]
pub struct WriteOnceError;

pub struct IsItUninit<T> {
    maybe_uninit: MaybeUninit<T>,
    uninit: AtomicBool,
}

impl<T> IsItUninit<T> {
    pub const fn new() -> IsItUninit<T> {
        IsItUninit {
            maybe_uninit: MaybeUninit::uninit(),
            uninit: AtomicBool::new(true),
        }
    }

    pub fn initialized(&self) -> bool {
        !self.uninit.load(Ordering::Relaxed)
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.try_get_mut()
            .expect("get_mut() called on uninitialized IsItUninit")
    }

    pub fn try_get_mut(&mut self) -> Option<&mut T> {
        if self.uninit.load(Ordering::Acquire) {
            None
        } else {
            Some(unsafe { self.maybe_uninit.assume_init_mut() })
        }
    }

    pub fn get_ref(&self) -> &T {
        self.try_get_ref()
            .expect("get_ref() called on uninitialized IsItUninit")
    }

    pub fn try_get_ref(&self) -> Option<&T> {
        if self.uninit.load(Ordering::Acquire) {
            None
        } else {
            Some(unsafe { self.maybe_uninit.assume_init_ref() })
        }
    }

    pub fn get(self) -> T {
        self.try_get()
            .expect("get() called on uninitialized IsItUninit")
    }

    pub fn try_get(self) -> Option<T> {
        if self.uninit.load(Ordering::Acquire) {
            None
        } else {
            Some(unsafe { self.maybe_uninit.assume_init() })
        }
    }

    pub fn write(&mut self, data: T) {
        self.maybe_uninit.write(data);
        self.uninit.swap(false, Ordering::AcqRel);
    }

    pub fn try_write_once(&mut self, data: T) -> Result<(), WriteOnceError> {
        self.uninit
            .compare_exchange_weak(true, false, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| WriteOnceError)?;

        self.maybe_uninit.write(data);

        Ok(())
    }

    pub fn write_once(&mut self, data: T) {
        self.try_write_once(data)
            .expect("write_once() called on initialized IsItUninit")
    }
}

impl<T> Default for IsItUninit<T> {
    fn default() -> Self {
        Self::new()
    }
}
