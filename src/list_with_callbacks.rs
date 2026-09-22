//! Corresponds to Java: `org/mpxj/ListWithCallbacks.java`

use std::fmt::{self, Debug, Formatter};

/// A list backed by `Vec` that triggers callbacks on insertion, removal, and replacement.
///
/// Corresponds to Java: `public abstract class ListWithCallbacks<T> extends AbstractList<T>`
///
/// # Differences from Java: callback fields instead of subclass overrides
///
/// Java is an abstract class, and subclasses (such as `ProjectEntityContainer`) are notified by overriding the `protected`
/// hooks `added` / `removed` / `replaced`. Rust has no inheritance, so instead the struct carries callbacks
/// via `Option<Box<dyn FnMut(..)>>`, registered by the owner through
/// methods such as [`ListWithCallbacks::set_added_callback`]; when unregistered, the behavior is exactly the same as the empty implementation
/// in Java (does nothing).
///
/// Callbacks fire **after** the list changes, with arguments matching the modified element:
///
/// | Java | Rust |
/// |---|---|
/// | `protected void added(T element)` | [`ListWithCallbacks::set_added_callback`] |
/// | `protected void removed(T element)` | [`ListWithCallbacks::set_removed_callback`] |
/// | `protected void replaced(T oldElement, T newElement)` | [`ListWithCallbacks::set_replaced_callback`] |
///
/// # Differences from Java: out-of-bounds no longer throws
///
/// Java's `get` / `set` / `remove` / `add(int, T)` throw
/// `IndexOutOfBoundsException` when out of bounds; the Rust side does not panic:
/// * [`get`](ListWithCallbacks::get) / [`remove`](ListWithCallbacks::remove) return `None` when out of bounds;
/// * [`set`](ListWithCallbacks::set) returns `None` when out of bounds and does not modify the list;
/// * [`add_at`](ListWithCallbacks::add_at) returns `false` when out of bounds and does not modify the list.
///
/// # Differences from Java:`Collections.sort`
///
/// Java's `AbstractList` has no sort method, so `ProjectEntityWithIDContainer` uses
/// `Collections.sort(this)`, which internally replaces elements one by one via `set(int, T)`, thereby triggering
/// the `replaced` callback. The Rust side's [`sort`](ListWithCallbacks::sort) sorts the internal `Vec` directly,
/// without triggering callbacks; for this crate's containers the `replaced` callback is always empty, so the behavior is equivalent.
pub struct ListWithCallbacks<T> {
/// The list contents.
    ///
    /// Corresponds to Java: `private final List<T> m_list = new ArrayList<>();`
    items: Vec<T>,
/// Callback for element addition.
    ///
    /// Corresponds to Java: `protected void added(T element)`
    added: Option<Box<dyn FnMut(&T)>>,
/// Callback for element removal.
    ///
    /// Corresponds to Java: `protected void removed(T element)`
    removed: Option<Box<dyn FnMut(&T)>>,
/// Callback for element replacement.
    ///
    /// Corresponds to Java: `protected void replaced(T oldElement, T newElement)`
    replaced: Option<Box<dyn FnMut(&T, &T)>>,
}

impl<T> Default for ListWithCallbacks<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ListWithCallbacks<T> {
/// Construct an empty list.
    ///
/// Corresponds to Java: the default constructor of `ArrayList`.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            added: None,
            removed: None,
            replaced: None,
        }
    }

/// Register the element addition callback.
    ///
/// Corresponds to Java: subclass overrides `protected void added(T element)`
    pub fn set_added_callback(&mut self, callback: Option<Box<dyn FnMut(&T)>>) {
        self.added = callback;
    }

/// Register the element removal callback.
    ///
/// Corresponds to Java: subclass overrides `protected void removed(T element)`
    pub fn set_removed_callback(&mut self, callback: Option<Box<dyn FnMut(&T)>>) {
        self.removed = callback;
    }

/// Register the element replacement callback.
    ///
/// Corresponds to Java: subclass overrides `protected void replaced(T oldElement, T newElement)`
    pub fn set_replaced_callback(&mut self, callback: Option<Box<dyn FnMut(&T, &T)>>) {
        self.replaced = callback;
    }

/// Clear the list. Consistent with Java: the `removed` callback is **not** triggered.
    ///
    /// Corresponds to Java: `@Override public void clear()`
    pub fn clear(&mut self) {
        self.items.clear();
    }

/// Get an element by index; returns `None` when out of bounds (Java throws `IndexOutOfBoundsException`).
    ///
    /// Corresponds to Java: `@Override public T get(int index)`
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

/// The number of elements.
    ///
    /// Corresponds to Java: `@Override public int size()`
    pub fn len(&self) -> usize {
        self.items.len()
    }

/// Whether the list is empty.
    ///
    /// Corresponds to Java: `@Override public boolean isEmpty()`
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

/// Replace the element at the specified index and return the old element that was replaced.
    ///
    /// Corresponds to Java: `@Override public T set(int index, T element)`
    ///
/// Triggers the `replaced` callback on success; returns `None` when out of bounds and does not modify the list.
    pub fn set(&mut self, index: usize, element: T) -> Option<T> {
        if index >= self.items.len() {
            return None;
        }

        let old_element = std::mem::replace(&mut self.items[index], element);

        if let Some(callback) = self.replaced.as_mut() {
            callback(&old_element, &self.items[index]);
        }

        Some(old_element)
    }

/// Append an element to the end of the list and trigger the `added` callback.
    ///
    /// Corresponds to Java: `@Override public boolean add(T e)`
    ///
/// Java always returns `true`; the Rust side returns nothing.
    pub fn add(&mut self, element: T) {
        self.items.push(element);

        let index = self.items.len() - 1;
        if let Some(callback) = self.added.as_mut() {
            callback(&self.items[index]);
        }
    }

/// Insert an element at the specified index and trigger the `added` callback.
    ///
    /// Corresponds to Java: `@Override public void add(int index, T element)`
    ///
/// When out of bounds (`index > len`) returns `false` and does not modify the list (Java throws
/// `IndexOutOfBoundsException`); returns `true` on a successful insertion.
    pub fn add_at(&mut self, index: usize, element: T) -> bool {
        if index > self.items.len() {
            return false;
        }

        self.items.insert(index, element);

        if let Some(callback) = self.added.as_mut() {
            callback(&self.items[index]);
        }

        true
    }

/// Remove the element at the specified index and return it, triggering the `removed` callback.
    ///
    /// Corresponds to Java: `@Override public T remove(int index)`
    ///
/// Returns `None` when out of bounds (Java throws `IndexOutOfBoundsException`).
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.items.len() {
            return None;
        }

        let removed = self.items.remove(index);

        if let Some(callback) = self.removed.as_mut() {
            callback(&removed);
        }

        Some(removed)
    }

/// Iterate over the list by index.
    ///
    /// Corresponds to Java: `AbstractList.iterator()`
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

/// Access the list contents as a slice.
    ///
/// Corresponds to Java: the list view of `AbstractList`.
    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

/// Sort in place by `Ord` (does not trigger callbacks; see the type documentation).
    ///
/// Corresponds to Java: `Collections.sort(this)` in `ProjectEntityWithIDContainer`
    pub fn sort(&mut self)
    where
        T: Ord,
    {
        self.items.sort();
    }
}

impl<T: Debug> Debug for ListWithCallbacks<T> {
/// Java's `AbstractList` provides `toString()`; the Rust side uses `Debug` to output the inner elements.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.items.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

/// Create a log that records callback events.
    fn log() -> Rc<RefCell<Vec<String>>> {
        Rc::new(RefCell::new(Vec::new()))
    }

/// Register three logging callbacks on the list.
    fn with_logging_callbacks(list: &mut ListWithCallbacks<i32>, log: Rc<RefCell<Vec<String>>>) {
        let added = Rc::clone(&log);
        list.set_added_callback(Some(Box::new(move |element| {
            added.borrow_mut().push(format!("added:{element}"));
        })));

        let removed = Rc::clone(&log);
        list.set_removed_callback(Some(Box::new(move |element| {
            removed.borrow_mut().push(format!("removed:{element}"));
        })));

        list.set_replaced_callback(Some(Box::new(move |old, new| {
            log.borrow_mut().push(format!("replaced:{old}->{new}"));
        })));
    }

    #[test]
    fn add_appends_and_notifies() {
        let mut list = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        list.add(1);
        list.add(2);

        assert_eq!(list.len(), 2);
        assert_eq!(list.get(0), Some(&1));
        assert_eq!(list.get(1), Some(&2));
        assert_eq!(list.get(2), None);
        assert_eq!(*log.borrow(), vec!["added:1", "added:2"]);
    }

    #[test]
    fn add_at_inserts_at_index() {
        let mut list = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        list.add(1);
        list.add(3);
        assert!(list.add_at(1, 2));

        assert_eq!(list.as_slice(), &[1, 2, 3]);
        assert_eq!(*log.borrow(), vec!["added:1", "added:3", "added:2"]);
    }

    #[test]
    fn add_at_out_of_range_is_ignored() {
        let mut list: ListWithCallbacks<i32> = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        assert!(!list.add_at(1, 1));
        assert!(list.is_empty());
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn set_returns_old_value_and_notifies() {
        let mut list = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        list.add(1);
        log.borrow_mut().clear();

        assert_eq!(list.set(0, 9), Some(1));
        assert_eq!(list.get(0), Some(&9));
        assert_eq!(*log.borrow(), vec!["replaced:1->9"]);

// Out of bounds does not modify the list and does not trigger callbacks
        assert_eq!(list.set(5, 10), None);
        assert_eq!(list.as_slice(), &[9]);
        assert_eq!(log.borrow().len(), 1);
    }

    #[test]
    fn remove_returns_element_and_notifies() {
        let mut list = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        list.add(1);
        list.add(2);
        log.borrow_mut().clear();

        assert_eq!(list.remove(0), Some(1));
        assert_eq!(list.as_slice(), &[2]);
        assert_eq!(*log.borrow(), vec!["removed:1"]);

// Out of bounds does not trigger callbacks
        assert_eq!(list.remove(5), None);
        assert_eq!(log.borrow().len(), 1);
    }

    #[test]
    fn clear_does_not_notify() {
        let mut list = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        list.add(1);
        list.add(2);
        log.borrow_mut().clear();

        list.clear();

        assert!(list.is_empty());
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn sort_orders_contents_without_notifying() {
        let mut list = ListWithCallbacks::new();
        let log = log();
        with_logging_callbacks(&mut list, Rc::clone(&log));

        list.add(3);
        list.add(1);
        list.add(2);
        log.borrow_mut().clear();

        list.sort();

        assert_eq!(list.as_slice(), &[1, 2, 3]);
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn callbacks_are_optional() {
        let mut list = ListWithCallbacks::new();

        list.add(1);
        assert_eq!(list.set(0, 2), Some(1));
        assert_eq!(list.remove(0), Some(2));
        assert!(list.is_empty());
    }
}
