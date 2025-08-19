use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::x86::idt::interrupt_control::{
    disable_interrupts, enable_interrupts, interrupts_enabled,
};

pub struct NegativeSendAndSync;

impl !Send for NegativeSendAndSync {}
impl !Sync for NegativeSendAndSync {}

pub struct MutexGuard<'a, T> {
    val: &'a mut T,
    locked: &'a AtomicUsize,
    traits: PhantomData<NegativeSendAndSync>,
    initial_interrupts: bool,
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.val
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.val
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.locked.store(0, Ordering::Release);
        if self.initial_interrupts {
            enable_interrupts();
        }
    }
}

pub struct Mutex<T> {
    val: UnsafeCell<T>,
    locked: AtomicUsize,
    traits: PhantomData<NegativeSendAndSync>,
}

impl<T> Mutex<T> {
    pub const fn new(val: T) -> Mutex<T> {
        Mutex {
            val: UnsafeCell::new(val),
            locked: AtomicUsize::new(0),
            traits: PhantomData,
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        let initial = interrupts_enabled();
        disable_interrupts();

        loop {
            unsafe {
                if self
                    .locked
                    .compare_exchange_weak(0, 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    spin_loop();
                    continue;
                }

                return MutexGuard {
                    val: &mut *self.val.get(),
                    locked: &self.locked,
                    traits: PhantomData,
                    initial_interrupts: initial,
                };
            }
        }
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Sync> Sync for Mutex<T> {}
