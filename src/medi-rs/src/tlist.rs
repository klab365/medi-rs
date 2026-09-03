//! Typed tuple primitives used by macro-generated mediators.
//!
//! Rust cannot provide a recursive `Get<T>` implementation for tuples because
//! the head-match and recursive implementations overlap. `Get` therefore also
//! carries an internal type-level position (`Here` or `There<I>`). Callers use
//! [`get`](crate::tlist::get), which lets the compiler infer that position from the requested type.

use core::marker::PhantomData;

/// Type-level position for the first item of a tuple list.
#[doc(hidden)]
pub struct Here;

/// Type-level position after the first item of a tuple list.
#[doc(hidden)]
pub struct There<I>(PhantomData<fn() -> I>);

/// Extract a cloned resource from a nested tuple list at a known position.
///
/// Macro-generated mediator code should use [`get`](crate::tlist::get) instead of naming `I`
/// directly.
#[doc(hidden)]
pub trait Get<T, I> {
    /// Clone the resource at this type-level position.
    fn get(&self) -> T;
}

impl<T, Tail> Get<T, Here> for (T, Tail)
where
    T: Clone,
{
    fn get(&self) -> T {
        self.0.clone()
    }
}

impl<T, Head, Tail, I> Get<T, There<I>> for (Head, Tail)
where
    Tail: Get<T, I>,
{
    fn get(&self) -> T {
        self.1.get()
    }
}

/// Clone a typed resource from a nested tuple list.
///
/// The type-level tuple position is inferred by the compiler. If `T` is not
/// registered in `R`, this function has no applicable [`Get`] implementation,
/// producing a compile-time error.
#[doc(hidden)]
pub fn get<T, I, R>(resources: &R) -> T
where
    R: Get<T, I>,
{
    resources.get()
}

#[cfg(test)]
mod tests {
    use super::get;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Repository(&'static str);

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Clock(u64);

    #[test]
    fn gets_head_resource() {
        let resources = (Repository("users"), (Clock(42), ()));

        let repository: Repository = get(&resources);

        assert_eq!(repository, Repository("users"));
    }

    #[test]
    fn gets_nested_resource() {
        let resources = (Repository("users"), (Clock(42), ()));

        let clock: Clock = get(&resources);

        assert_eq!(clock, Clock(42));
    }
}
