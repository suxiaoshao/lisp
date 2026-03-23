use crate::root::Root;
use gc_arena::{Arena, Collect, Gc, Mutation, RefLock};
use gc_arena_derive::Collect;

/// Type alias for the GC arena
pub type GC<'gc> = Arena<Root>;

/// Type alias for GC-wrapped RefLock
pub type Locked<'gc, T> = Gc<'gc, RefLock<T>>;

/// Macro for convenient allocation
#[macro_export]
macro_rules! gc_alloc {
    ($arena:expr, $value:expr) => {
        Gc::new($arena, $value)
    };
}
