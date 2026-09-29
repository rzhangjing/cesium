//! cesium-event：类型安全的事件系统。
//! 领域层 —— 纯 Rust，无框架依赖。
//!
//! CesiumJS 映射：`packages/engine/Source/Core/Event.js`

use std::cell::RefCell;
use std::collections::HashMap;

/// 事件监听器的唯一标识符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListenerId(u64);

/// 监听器映射的类型别名，用于降低复杂度。
type ListenerMap<Args> = HashMap<u64, Box<dyn Fn(&Args)>>;

/// 一个可以有多个监听器的通用事件。
/// 映射到 CesiumJS 的 `Event`
///
/// 类型参数 `Args` 是传递给监听器的参数类型元组。
pub struct Event<Args: Clone> {
    listeners: RefCell<ListenerMap<Args>>,
    next_id: RefCell<u64>,
}

impl<Args: Clone> Event<Args> {
    /// 创建一个空事件。
    pub fn new() -> Self {
        Self {
            listeners: RefCell::new(HashMap::new()),
            next_id: RefCell::new(0),
        }
    }

    /// 返回当前已订阅的监听器数量。
    /// 映射到 `Event.numberOfListeners`
    pub fn number_of_listeners(&self) -> usize {
        self.listeners.borrow().len()
    }

    /// 若没有任何监听器则返回 true。
    pub fn is_empty(&self) -> bool {
        self.listeners.borrow().is_empty()
    }

    /// 注册一个回调函数，只要事件被触发就会执行。
    /// 映射到 `Event.addEventListener`
    ///
    /// 返回一个可用于移除该监听器的 `ListenerId`。
    pub fn add_listener<F>(&self, listener: F) -> ListenerId
    where
        F: Fn(&Args) + 'static,
    {
        let mut next_id = self.next_id.borrow_mut();
        let id = *next_id;
        *next_id += 1;

        self.listeners.borrow_mut().insert(id, Box::new(listener));
        ListenerId(id)
    }

    /// 注销先前已注册的回调。
    /// 映射到 `Event.removeEventListener`
    ///
    /// 若监听器被移除则返回 true。
    pub fn remove_listener(&self, id: ListenerId) -> bool {
        self.listeners.borrow_mut().remove(&id.0).is_some()
    }

    /// 触发事件：以给定参数依次调用每个已注册的监听器。
    /// 映射到 `Event.raiseEvent`
    pub fn raise(&self, args: &Args) {
        let listeners = self.listeners.borrow();
        for listener in listeners.values() {
            listener(args);
        }
    }

    /// 移除所有监听器。
    pub fn clear(&self) {
        self.listeners.borrow_mut().clear();
    }
}

impl<Args: Clone> Default for Event<Args> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Args: Clone> std::fmt::Debug for Event<Args> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Event")
            .field("listener_count", &self.number_of_listeners())
            .finish()
    }
}

/// 一个无参数的简单事件。
pub type SimpleEvent = Event<()>;

impl SimpleEvent {
    /// 以无参数触发事件。
    pub fn raise_simple(&self) {
        self.raise(&());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn test_add_and_raise() {
        let event: Event<i32> = Event::new();
        let received = Rc::new(Cell::new(0));
        let received_clone = received.clone();

        event.add_listener(move |val| {
            received_clone.set(*val);
        });

        event.raise(&42);
        assert_eq!(received.get(), 42);
    }

    #[test]
    fn test_multiple_listeners() {
        let event: Event<i32> = Event::new();
        let sum = Rc::new(Cell::new(0));

        let sum1 = sum.clone();
        event.add_listener(move |val| {
            sum1.set(sum1.get() + val);
        });

        let sum2 = sum.clone();
        event.add_listener(move |val| {
            sum2.set(sum2.get() + val * 2);
        });

        event.raise(&10);
        assert_eq!(sum.get(), 30); // 10 + 20
    }

    #[test]
    fn test_remove_listener() {
        let event: Event<i32> = Event::new();
        let count = Rc::new(Cell::new(0));
        let count_clone = count.clone();

        let id = event.add_listener(move |_| {
            count_clone.set(count_clone.get() + 1);
        });

        event.raise(&0);
        assert_eq!(count.get(), 1);

        assert!(event.remove_listener(id));
        event.raise(&0);
        assert_eq!(count.get(), 1); // 不应再递增
    }

    #[test]
    fn test_number_of_listeners() {
        let event: Event<()> = Event::new();
        assert_eq!(event.number_of_listeners(), 0);

        let id1 = event.add_listener(|_| {});
        assert_eq!(event.number_of_listeners(), 1);

        let _id2 = event.add_listener(|_| {});
        assert_eq!(event.number_of_listeners(), 2);

        event.remove_listener(id1);
        assert_eq!(event.number_of_listeners(), 1);
    }

    #[test]
    fn test_simple_event() {
        let event = SimpleEvent::new();
        let fired = Rc::new(Cell::new(false));
        let fired_clone = fired.clone();

        event.add_listener(move |_| {
            fired_clone.set(true);
        });

        assert!(!fired.get());
        event.raise_simple();
        assert!(fired.get());
    }

    #[test]
    fn test_clear() {
        let event: Event<()> = Event::new();
        event.add_listener(|_| {});
        event.add_listener(|_| {});
        assert_eq!(event.number_of_listeners(), 2);

        event.clear();
        assert_eq!(event.number_of_listeners(), 0);
    }
}
