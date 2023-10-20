use crate::VK_NULL_HANDLE;
use std::{borrow, fmt, ops};
#[cfg(feature = "nightly")]
use std::{
    collections::HashMap,
    hash::BuildHasherDefault,
    sync::{Arc, Mutex},
};

use copyless::{BoxAllocation, BoxHelper};

#[cfg(feature = "nightly")]
use lazy_static::lazy_static;

#[cfg(feature = "nightly")]
lazy_static! {
    static ref REGISTRY: Arc<Mutex<HashMap<usize, &'static str, BuildHasherDefault<fxhash::FxHasher>>>> =
        Arc::new(Mutex::new(HashMap::default()));
}

#[repr(C)]
pub struct Handle<T>(*mut T);

#[cfg(feature = "nightly")]
impl Handle<()> {
    pub fn report_leaks() {
        println!("Leaked handles:");
        let mut map = REGISTRY.lock().unwrap();
        for (_, type_id) in map.drain() {
            println!("\t{:?}", type_id);
        }
    }
}

pub struct HandleAllocation<T>(BoxAllocation<T>);

impl<T> HandleAllocation<T> {
    #[inline(always)]
    pub fn init(self, value: T) -> Handle<T> {
        let ptr = Box::into_raw(self.0.init(value));
        #[cfg(feature = "nightly")]
        {
            use std::intrinsics::type_name;
            let name = type_name::<T>();
            REGISTRY.lock().unwrap().insert(ptr as _, name);
        }
        Handle(ptr)
    }
}

impl<T: 'static> Handle<T> {
    pub fn alloc() -> HandleAllocation<T> {
        HandleAllocation(Box::alloc())
    }

    // Note: ideally this constructor isn't used
    pub fn new(value: T) -> Self {
        Self::alloc().init(value)
    }

    pub fn null() -> Self {
        Handle(VK_NULL_HANDLE as *mut _)
    }

    pub fn unbox(self) -> Option<T> {
        if self.0 == VK_NULL_HANDLE as *mut T {
            None
        } else {
            #[cfg(feature = "nightly")]
            {
                REGISTRY.lock().unwrap().remove(&(self.0 as _)).unwrap();
            }
            Some(*unsafe { Box::from_raw(self.0) })
        }
    }

    pub fn as_ref(&self) -> Option<&T> {
        unsafe { self.0.as_ref() }
    }

    pub fn as_mut(&self) -> Option<&mut T> {
        unsafe { self.0.as_mut() }
    }
}

impl<T> Handle<T> {
    #[cfg(feature = "nightly")]
    #[inline]
    fn check(&self) {
        assert!(REGISTRY.lock().unwrap().contains_key(&(self.0 as _)));
    }
    #[cfg(not(feature = "nightly"))]
    #[inline]
    fn check(&self) {
        debug_assert!(!self.0.is_null());
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> ops::Deref for Handle<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.check();
        unsafe { &*self.0 }
    }
}

impl<T> ops::DerefMut for Handle<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.check();
        unsafe { &mut *self.0 }
    }
}

impl<T> borrow::Borrow<T> for Handle<T> {
    fn borrow(&self) -> &T {
        self.check();
        unsafe { &*self.0 }
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Handle({:p})", self.0)
    }
}

#[cfg(feature = "dispatch")]
pub use self::dispatch::DispatchHandle;
#[cfg(not(feature = "dispatch"))]
pub type DispatchHandle<T> = Handle<T>;

#[cfg(feature = "dispatch")]
mod dispatch {
    use crate::VK_NULL_HANDLE;
    use copyless::{BoxAllocation, BoxHelper};
    use std::{borrow, fmt, ops};

    const ICD_LOADER_MAGIC: u64 = 0x01CDC0DE;

    #[repr(C)]
    pub struct DispatchHandle<T>(*mut (u64, T));

    pub struct DisplatchHandleAllocation<T>(BoxAllocation<(u64, T)>);

    impl<T> DisplatchHandleAllocation<T> {
        #[inline(always)]
        pub fn init(self, value: T) -> DispatchHandle<T> {
            let ptr = Box::into_raw(self.0.init((ICD_LOADER_MAGIC, value)));
            DispatchHandle(ptr)
        }
    }

    impl<T> DispatchHandle<T> {
        pub fn alloc() -> DisplatchHandleAllocation<T> {
            DisplatchHandleAllocation(Box::alloc())
        }

        pub fn new(value: T) -> Self {
            Self::alloc().init(value)
        }

        pub fn null() -> Self {
            DispatchHandle(VK_NULL_HANDLE as *mut _)
        }

        pub fn unbox(self) -> Option<T> {
            if self.0 == VK_NULL_HANDLE as *mut (u64, T) {
                None
            } else {
                Some(unsafe { Box::from_raw(self.0) }.1)
            }
        }

        pub fn as_ref(&self) -> Option<&T> {
            if self.0 == VK_NULL_HANDLE as *mut (u64, T) {
                None
            } else {
                Some(unsafe { &(*self.0).1 })
            }
        }
    }

    impl<T> Clone for DispatchHandle<T> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<T> Copy for DispatchHandle<T> {}

    impl<T> ops::Deref for DispatchHandle<T> {
        type Target = T;
        fn deref(&self) -> &T {
            unsafe { &(*self.0).1 }
        }
    }

    impl<T> ops::DerefMut for DispatchHandle<T> {
        fn deref_mut(&mut self) -> &mut T {
            unsafe { &mut (*self.0).1 }
        }
    }

    impl<T> borrow::Borrow<T> for DispatchHandle<T> {
        fn borrow(&self) -> &T {
            unsafe { &(*self.0).1 }
        }
    }

    impl<T> PartialEq for DispatchHandle<T> {
        fn eq(&self, other: &Self) -> bool {
            self.0.eq(&other.0)
        }
    }

    impl<T> fmt::Debug for DispatchHandle<T> {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "DispatchHandle({:p})", self.0)
        }
    }
}
