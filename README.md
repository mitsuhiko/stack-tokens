# Stack Tokens

[![Crates.io](https://img.shields.io/crates/d/stack-tokens.svg)](https://crates.io/crates/stack-tokens)
[![License](https://img.shields.io/github/license/mitsuhiko/stack-tokens)](https://github.com/mitsuhiko/stack-tokens/main/LICENSE)
[![Documentation](https://docs.rs/stack-tokens/badge.svg)](https://docs.rs/stack-tokens)

This library implements stack tokens which can be used to safely borrow values
with stack-local lifetimes.

`StackToken`s are zero sized objects that can be placed on the call stack with the
`stack_token!` macro which then can be used to safely borrow from places such as
thread local storage with reduced lifetimes.

Without stack tokens the only safe API for such constructs are callback based
such as the `LocalKey::with` API. This problem however is not always restricted to
thread local storage directly as some APIs are internally constrained by similar
challenges.

The problem usually appears when a proxy object wants to lend out some memory
but it does not have a better lifetime than itself to constrain the value, but
it does not directly own the value it’s trying to lend out. As a Rust programmer
one is enticed to try to constrain it by the lifetime of &self but thanks to
`Box::leak` that lifetime can become &'static.

For more information see [the the blog post describing the concept](https://lucumr.pocoo.org/2022/11/23/stack-tokens/).

```rust
use stack_tokens::{stack_token, RefCellLocalKeyExt};
use std::cell::RefCell;

thread_local! {
    static VEC: RefCell<Vec<i32>> = RefCell::default();
}

// places a token on the stack.
stack_token!(scope);

// you can now directly deref the thread local without closures
VEC.as_mut(scope).push(42);
assert_eq!(VEC.as_ref(scope).len(), 1);
```