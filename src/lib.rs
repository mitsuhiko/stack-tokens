//! This library implements stack tokens which can be used to safely borrow
//! values with stack-local lifetimes.
//!
//! [`StackToken`]s are zero sized objects that can be placed on the call
//! stack with the [`stack_token!`] macro which then can be used to safely
//! borrow from places such as thread local storage with reduced lifetimes.
//!
//! Without stack tokens the only safe API for such constructs are callback
//! based such as the [`LocalKey::with`](std::thread::LocalKey::with) API.
//! This problem however is not always restricted to thread local storage
//! directly as some APIs are internally constrained by similar challenges.
//!
//! The problem usually appears when a proxy object wants to lend out some
//! memory but it does not have a better lifetime than itself to constrain
//! the value, but it does not directly own the value it's trying to lend
//! out.  As a Rust programmer one is enticed to try to constrain it by
//! the lifetime of `&self` but thanks to [`Box::leak`] that lifetime can
//! become `&'static`.
//!
//! For more information see the [the blog post describing the concept](https://lucumr.pocoo.org/2022/11/23/stack-tokens/).
//!
//! # Ref Cells
//!
//! This example shows how stack tokens can be used with the
//! [`RefCellLocalKeyExt`] extension trait to directly borrow into [`RefCell`]s
//! in a thread local.
//!
//! ```
//! use stack_tokens::{stack_token, RefCellLocalKeyExt};
//! use std::cell::RefCell;
//!
//! thread_local! {
//!     static VEC: RefCell<Vec<i32>> = RefCell::default();
//! }
//!
//! // places a token on the stack.
//! stack_token!(scope);
//!
//! // you can now directly deref the thread local without closures
//! VEC.as_mut(scope).push(42);
//! assert_eq!(VEC.as_ref(scope).len(), 1);
//! ```
//!
//! # Basic Use
//!
//! This example shows how stack tokens can be used with the [`LocalKeyExt`]
//! extension trait to get stack local borrows from a standard library thread
//! local.
//!
//! ```
//! use stack_tokens::{stack_token, LocalKeyExt};
//! use std::sync::atomic::{AtomicUsize, Ordering};
//!
//! thread_local! {
//!     static COUNTER: AtomicUsize = AtomicUsize::new(0);
//! }
//!
//! // places a token on the stack
//! stack_token!(scope);
//!
//! // borrow can be used to get a stack local reference
//! COUNTER.borrow(scope).fetch_add(1, Ordering::Acquire);
//! assert_eq!(COUNTER.borrow(scope).load(Ordering::Acquire), 1);
//! ```
//!
//! # Implementing Stack Local APIs
//!
//! To implement your own methods that use stack tokens introduce a new lifetime
//! (eg: `'stack`) and constrain both `&self` and the passed token with it:
//!
//! ```
//! use stack_tokens::StackToken;
//! use std::marker::PhantomData;
//!
//! struct MyTls<T>(PhantomData::<T>);
//!
//! impl<T> MyTls<T> {
//!     pub fn get<'stack>(&'stack self, token: &'stack StackToken) -> &'stack T {
//!         let _ = token;
//!         todo!()
//!     }
//! }
//! ```
use std::cell::{Ref, RefCell, RefMut};
use std::marker::PhantomData;
use std::mem::transmute;
use std::thread::LocalKey;

/// A token to bind lifetimes to a specific stack.
///
/// For more information see [`stack_token`].
pub struct StackToken {
    _marker: PhantomData<*const ()>,
}

impl StackToken {
    #[doc(hidden)]
    pub unsafe fn __private_new() -> StackToken {
        StackToken {
            _marker: PhantomData,
        }
    }
}

/// Creates a new [`StackToken`] with a given name on the stack.
#[macro_export]
macro_rules! stack_token {
    ($name:ident) => {
        #[allow(unsafe_code)]
        let $name = &unsafe { $crate::StackToken::__private_new() };
    };
}

/// Adds [`StackToken`] support to the standard library's [`LocalKey`].
pub trait LocalKeyExt<T> {
    /// Borrows the value from the TLS with a [`StackToken`].
    fn borrow<'stack>(&'static self, token: &'stack StackToken) -> &'stack T;
}

impl<T: 'static> LocalKeyExt<T> for LocalKey<T> {
    fn borrow<'stack>(&'static self, token: &'stack StackToken) -> &'stack T {
        let _ = token;
        self.with(|value| unsafe { transmute::<&T, &'stack T>(value) })
    }
}

/// Additional utility methods to [`LocalKey`]s holding [`RefCell`] values.
///
/// This extension traits provides the two methods [`as_ref`](Self::as_ref)
/// and [`as_mut`](Self::as_mut) that let you directly borrow into the
/// contained [`RefCell`] with a [`StackToken`].
pub trait RefCellLocalKeyExt<T> {
    /// Acquires a reference to the contained value.
    fn as_ref<'stack>(&'static self, token: &'stack StackToken) -> Ref<'stack, T>;

    /// Acquires a mutable reference to the contained value.
    fn as_mut<'stack>(&'static self, token: &'stack StackToken) -> RefMut<'stack, T>;
}

impl<T: 'static> RefCellLocalKeyExt<T> for LocalKey<RefCell<T>> {
    fn as_ref<'stack>(&'static self, token: &'stack StackToken) -> Ref<'stack, T> {
        self.borrow(token).borrow()
    }

    fn as_mut<'stack>(&'static self, token: &'stack StackToken) -> RefMut<'stack, T> {
        self.borrow(token).borrow_mut()
    }
}

#[test]
fn test_tls_basic() {
    use crate::stack_token;
    use std::cell::RefCell;

    thread_local! { static FOO: RefCell<u32> = RefCell::default(); }

    stack_token!(scope);
    *FOO.borrow(scope).borrow_mut() += 1;
    assert_eq!(*FOO.borrow(scope).borrow(), 1);
}

#[test]
fn test_tls_ref_cell() {
    use crate::stack_token;
    use std::cell::RefCell;

    thread_local! { static FOO: RefCell<u32> = RefCell::default(); }

    stack_token!(scope);
    *FOO.as_mut(scope) += 1;
    assert_eq!(*FOO.as_ref(scope), 1);
}
