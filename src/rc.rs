// SPDX-License-Identifier: MIT

//! Reference counting with iterative destruction of uniquely owned graphs.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::rc::{Rc as StdRc, Weak};

thread_local! {
    static DRAINING: Cell<bool> = const { Cell::new(false) };
    static PENDING: RefCell<Vec<StdRc<dyn Any>>> = const { RefCell::new(Vec::new()) };
}

// The empty state lets Drop and try_unwrap take ownership using safe Rust.
// The nullable slot uses Rc's pointer niche; the size is checked below.
pub struct Rc<T: 'static>(Option<ManuallyDrop<StdRc<T>>>);

impl<T: 'static> Rc<T> {
    fn inner(&self) -> &StdRc<T> {
        self.0.as_deref().expect("live Rc")
    }
    fn inner_mut(&mut self) -> &mut StdRc<T> {
        self.0.as_deref_mut().expect("live Rc")
    }
    pub fn new(value: T) -> Self {
        Self(Some(ManuallyDrop::new(StdRc::new(value))))
    }
    pub fn as_ptr(this: &Self) -> *const T {
        StdRc::as_ptr(this.inner())
    }
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        StdRc::ptr_eq(this.inner(), other.inner())
    }
    pub fn strong_count(this: &Self) -> usize {
        StdRc::strong_count(this.inner())
    }
    pub fn downgrade(this: &Self) -> Weak<T> {
        StdRc::downgrade(this.inner())
    }
    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        StdRc::get_mut(this.inner_mut())
    }
    pub fn make_mut(this: &mut Self) -> &mut T
    where
        T: Clone,
    {
        StdRc::make_mut(this.inner_mut())
    }
    pub fn try_unwrap(this: Self) -> Result<T, Self> {
        let mut this = this;
        let inner = ManuallyDrop::into_inner(this.0.take().expect("live Rc"));
        StdRc::try_unwrap(inner).map_err(|inner| Self(Some(ManuallyDrop::new(inner))))
    }
}

impl<T: 'static> From<T> for Rc<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
impl<T: 'static> AsRef<T> for Rc<T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: 'static> Clone for Rc<T> {
    fn clone(&self) -> Self {
        Self(Some(ManuallyDrop::new(StdRc::clone(self.inner()))))
    }
}
impl<T: 'static> Deref for Rc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.inner()
    }
}
impl<T: Default + 'static> Default for Rc<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}
impl<T: fmt::Debug + 'static> fmt::Debug for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}
impl<T: fmt::Display + 'static> fmt::Display for Rc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
impl<T: PartialEq + 'static> PartialEq for Rc<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl<T: Eq + 'static> Eq for Rc<T> {}
impl<T: PartialOrd + 'static> PartialOrd for Rc<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(&**other)
    }
}
impl<T: Ord + 'static> Ord for Rc<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (**self).cmp(&**other)
    }
}
impl<T: Hash + 'static> Hash for Rc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

struct DrainGuard;
impl Drop for DrainGuard {
    fn drop(&mut self) {
        let _ = DRAINING.try_with(|active| active.set(false));
    }
}

impl<T: 'static> Drop for Rc<T> {
    fn drop(&mut self) {
        // try_unwrap leaves an empty slot before its wrapper is dropped.
        let Some(inner) = self.0.take() else { return };
        let inner = ManuallyDrop::into_inner(inner);
        if StdRc::strong_count(&inner) != 1 {
            drop(inner);
            return;
        }
        match DRAINING.try_with(|active| {
            if active.get() {
                false
            } else {
                active.set(true);
                true
            }
        }) {
            Ok(true) => {
                let _guard = DrainGuard;
                drop(inner);
                while let Ok(Some(next)) = PENDING.try_with(|pending| {
                    let mut pending = pending.borrow_mut();
                    let next = pending.pop();
                    // Return unused queue storage gradually during wide drains.
                    if pending.capacity() > 4096 && pending.len() == pending.capacity() / 4 {
                        let capacity = pending.len().max(4096);
                        pending.shrink_to(capacity);
                    }
                    next
                }) {
                    drop(next);
                }
                let _ = PENDING.try_with(|pending| {
                    let mut pending = pending.borrow_mut();
                    if pending.capacity() > 4096 {
                        *pending = Vec::new();
                    }
                });
            }
            Ok(false) => {
                let mut pending: Option<StdRc<dyn Any>> = Some(inner);
                let _ = PENDING.try_with(|queue| {
                    queue.borrow_mut().push(pending.take().unwrap());
                });
                drop(pending);
            }
            Err(_) => drop(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Rc;
    use crate::value::{DictStorage, Scope, Value};
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

    fn small_stack(f: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(f)
            .unwrap()
            .join()
            .unwrap();
    }

    struct Node(Option<Rc<Node>>);

    #[test]
    fn million_nodes_drop_on_small_stack() {
        small_stack(|| {
            let mut node = Rc::new(Node(None));
            for _ in 0..1_000_000 {
                node = Rc::new(Node(Some(node)));
            }
            assert!((*node).0.is_some());
            drop(node);
        });
    }

    #[test]
    fn million_values_drop_on_small_stack() {
        small_stack(|| {
            for kind in 0..3 {
                let mut value = Value::None;
                for _ in 0..1_000_000 {
                    value = match kind {
                        0 => Value::List(Rc::new(RefCell::new(vec![value]))),
                        1 => Value::Dict(Rc::new(RefCell::new(DictStorage::new(vec![(
                            Value::None,
                            value,
                        )])))),
                        _ => {
                            let mut values = HashMap::new();
                            values.insert(String::from("x"), value);
                            let scope: Scope = Rc::new(RefCell::new(values));
                            Value::ScopeDict(scope)
                        }
                    };
                }
                drop(value);
            }
        });
    }

    #[test]
    fn shared_owner_survives() {
        small_stack(|| {
            let shared = Rc::new(RefCell::new(vec![Value::Number(42)]));
            let mut value = Value::List(shared.clone());
            for _ in 0..1_000_000 {
                value = Value::List(Rc::new(RefCell::new(vec![value])));
            }
            drop(value);
            assert_eq!(Rc::strong_count(&shared), 1);
            assert!(matches!(&shared.borrow()[..], [Value::Number(42)]));
        });
    }

    #[test]
    fn wide_drop_releases_pending_capacity() {
        let parent = Rc::new((0..100_000).map(Rc::new).collect::<Vec<_>>());
        drop(parent);
        let capacity = super::PENDING.with(|pending| pending.borrow().capacity());
        assert!(capacity <= 4096, "pending capacity: {capacity}");
    }

    #[test]
    fn drain_shrinks_large_queue_before_last_items() {
        struct Tracer {
            id: usize,
            observed: std::rc::Rc<Cell<Option<(usize, usize)>>>,
        }
        impl Drop for Tracer {
            fn drop(&mut self) {
                if self.id == 4096 {
                    super::PENDING.with(|pending| {
                        let pending = pending.borrow();
                        self.observed.set(Some((pending.len(), pending.capacity())));
                    });
                }
            }
        }

        let observed = std::rc::Rc::new(Cell::new(None));
        let parent = Rc::new(
            (0..16_384)
                .map(|id| {
                    Rc::new(Tracer {
                        id,
                        observed: observed.clone(),
                    })
                })
                .collect::<Vec<_>>(),
        );
        drop(parent);
        let (len, capacity) = observed.get().unwrap();
        assert_eq!(len, 4096);
        assert!(capacity <= 4096, "pending capacity: {capacity}");
    }

    #[test]
    fn drain_preserves_drop_order_with_nested_children() {
        struct Tracer {
            id: usize,
            log: std::rc::Rc<RefCell<Vec<usize>>>,
            _child: Option<Rc<Tracer>>,
        }
        impl Drop for Tracer {
            fn drop(&mut self) {
                self.log.borrow_mut().push(self.id);
            }
        }

        let log = std::rc::Rc::new(RefCell::new(Vec::new()));
        let parent = Rc::new(
            (0..10_000)
                .map(|id| {
                    Rc::new(Tracer {
                        id: 2 * id,
                        log: log.clone(),
                        _child: Some(Rc::new(Tracer {
                            id: 2 * id + 1,
                            log: log.clone(),
                            _child: None,
                        })),
                    })
                })
                .collect::<Vec<_>>(),
        );
        drop(parent);
        let expected: Vec<_> = (0..10_000)
            .rev()
            .flat_map(|id| [2 * id, 2 * id + 1])
            .collect();
        assert_eq!(*log.borrow(), expected);
    }

    #[test]
    fn drain_resumes_after_a_drop_panics() {
        struct Tracer {
            id: usize,
            count: std::rc::Rc<Cell<usize>>,
        }
        impl Drop for Tracer {
            fn drop(&mut self) {
                self.count.set(self.count.get() + 1);
                assert_ne!(self.id, 9_998, "intentional drop panic");
            }
        }

        let count = std::rc::Rc::new(Cell::new(0));
        let parent = Rc::new(
            (0..10_000)
                .map(|id| {
                    Rc::new(Tracer {
                        id,
                        count: count.clone(),
                    })
                })
                .collect::<Vec<_>>(),
        );
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(parent))).is_err());
        drop(Rc::new(()));
        assert_eq!(count.get(), 10_000);
        super::PENDING.with(|pending| assert!(pending.borrow().is_empty()));
    }

    #[test]
    fn pending_values_drop_on_thread_exit_after_a_panic() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct Tracer {
            count: Arc<AtomicUsize>,
            panic: bool,
        }
        impl Drop for Tracer {
            fn drop(&mut self) {
                self.count.fetch_add(1, Ordering::Relaxed);
                assert!(!self.panic, "intentional drop panic");
            }
        }

        let count = Arc::new(AtomicUsize::new(0));
        let thread_count = count.clone();
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(move || {
                let mut values: Vec<_> = (0..1_000_000)
                    .map(|_| {
                        Rc::new(Tracer {
                            count: thread_count.clone(),
                            panic: false,
                        })
                    })
                    .collect();
                values.push(Rc::new(Tracer {
                    count: thread_count,
                    panic: true,
                }));
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        drop(Rc::new(values))
                    }))
                    .is_err()
                );
            })
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(count.load(Ordering::Relaxed), 1_000_001);
    }

    #[test]
    fn repeated_traceback_chains_release_their_nodes() {
        use crate::value::TracebackNode;

        for _ in 0..3 {
            let mut node = Rc::new(TracebackNode { next: None });
            let tail = Rc::downgrade(&node);
            for _ in 0..100_000 {
                node = Rc::new(TracebackNode { next: Some(node) });
            }
            let head = Rc::downgrade(&node);
            drop(node);
            assert!(head.upgrade().is_none());
            assert!(tail.upgrade().is_none());
            super::PENDING.with(|pending| {
                assert!(pending.borrow().is_empty());
                assert!(pending.borrow().capacity() <= 4096);
            });
        }
    }

    #[test]
    fn wide_drain_releases_storage_after_growing() {
        struct Parent {
            children: Vec<Rc<u8>>,
            observed_capacity: std::rc::Rc<Cell<usize>>,
        }
        impl Drop for Parent {
            fn drop(&mut self) {
                self.children.clear();
                self.observed_capacity
                    .set(super::PENDING.with(|pending| pending.borrow().capacity()));
            }
        }

        let capacity = std::rc::Rc::new(Cell::new(0));
        drop(Rc::new(Parent {
            children: (0..100_000).map(|_| Rc::new(0)).collect(),
            observed_capacity: capacity.clone(),
        }));
        let after = super::PENDING.with(|pending| pending.borrow().capacity());
        assert!(capacity.get() >= 100_000);
        assert!(after <= 4096);
    }

    #[test]
    fn thread_exit_after_panic_preserves_pending_drop_order() {
        use std::sync::{Arc, Mutex};

        struct Tracer {
            id: usize,
            log: Arc<Mutex<Vec<usize>>>,
            child: Option<Rc<Tracer>>,
            panic: bool,
        }
        impl Drop for Tracer {
            fn drop(&mut self) {
                self.log.lock().unwrap().push(self.id);
                assert!(!self.panic, "intentional drop panic");
                drop(self.child.take());
            }
        }

        let log = Arc::new(Mutex::new(Vec::new()));
        let thread_log = log.clone();
        std::thread::spawn(move || {
            let mut values: Vec<_> = (0..5000)
                .map(|id| {
                    Rc::new(Tracer {
                        id,
                        log: thread_log.clone(),
                        child: (id == 0).then(|| {
                            Rc::new(Tracer {
                                id: 5000,
                                log: thread_log.clone(),
                                child: None,
                                panic: false,
                            })
                        }),
                        panic: false,
                    })
                })
                .collect();
            values.push(Rc::new(Tracer {
                id: 5001,
                log: thread_log,
                child: None,
                panic: true,
            }));
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    drop(Rc::new(values));
                }))
                .is_err()
            );
        })
        .join()
        .unwrap();
        let expected: Vec<_> = [5001, 0, 5000].into_iter().chain(1..5000).collect();
        assert_eq!(*log.lock().unwrap(), expected);
    }

    #[test]
    fn same_size_as_standard_rc() {
        assert_eq!(
            std::mem::size_of::<Rc<u8>>(),
            std::mem::size_of::<std::rc::Rc<u8>>()
        );
    }

    #[test]
    fn unwrap_and_unique_mutation_respect_weak_and_strong_owners() {
        let mut value = Rc::new(vec![1]);
        let weak = Rc::downgrade(&value);
        assert!(Rc::get_mut(&mut value).is_none());
        Rc::make_mut(&mut value).push(2);
        assert!(weak.upgrade().is_none());
        let other = value.clone();
        let value = Rc::try_unwrap(value).unwrap_err();
        assert!(Rc::ptr_eq(&value, &other));
        drop(other);
        assert_eq!(Rc::try_unwrap(value).unwrap(), vec![1, 2]);
    }

    #[test]
    fn panic_during_drain_resets_guard() {
        small_stack(|| {
            struct PanicOnce(Rc<Cell<bool>>);
            impl Drop for PanicOnce {
                fn drop(&mut self) {
                    if !self.0.replace(true) {
                        panic!("intentional drop panic");
                    }
                }
            }
            struct Parent {
                _child: Rc<PanicOnce>,
            }
            let flag = Rc::new(Cell::new(false));
            let parent = Rc::new(Parent {
                _child: Rc::new(PanicOnce(flag.clone())),
            });
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(parent))).is_err()
            );
            assert!(flag.get());
            let mut node = Rc::new(Node(None));
            for _ in 0..1_000_000 {
                node = Rc::new(Node(Some(node)));
            }
            drop(node);
            super::DRAINING.with(|active| assert!(!active.get()));
            super::PENDING.with(|pending| assert!(pending.borrow().is_empty()));
        });
    }
}
