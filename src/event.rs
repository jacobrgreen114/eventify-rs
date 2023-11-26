//
// Copyright 2023 Jacob R. Green
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use std::cell::UnsafeCell;
use std::sync::{Arc, Mutex, Weak};
use crate::Leak;

#[derive(Debug, Default)]
struct EventInner<Args> {
    callbacks: Vec<Arc<UnsafeCell<dyn FnMut(&Args) -> ()>>>,
}

/// A thread-safe event that can be hooked into.
///
/// # Example
/// ```rust
/// use eventify::event::*;
///
/// fn main() {
///     let event = Event::new();
///     let hook = event.hook(|args: &i32| {
///         println!("Event fired with args: {}", args);
///     });
///     event.invoke(&42);
/// }
/// ```
#[derive(Debug, Default)]
pub struct Event<Args = ()> {
    inner: Arc<Mutex<EventInner<Args>>>,
}

impl<Args> Event<Args> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(EventInner {
                callbacks: Vec::new(),
            })),
        }
    }

    /// Hooks a callback into the event, returning a hook that can be used to remove the hook.
    #[must_use]
    #[inline(always)]
    pub fn hook(&self, callback: impl FnMut(&Args) + 'static) -> Hook<Args> {
        self.hook_internal(Arc::new(UnsafeCell::new(callback)))
    }

    fn hook_internal(&self, callback: Arc<UnsafeCell<dyn FnMut(&Args) -> ()>>) -> Hook<Args> {
        let mut inner = self.inner.lock().unwrap();

        let weak_inner = Arc::downgrade(&self.inner);
        let weak_callback = Arc::downgrade(&callback);

        inner.callbacks.push(callback);
        Hook {
            data: Some(EventHookData {
                inner: weak_inner,
                callback: weak_callback,
            }),
        }
    }

    /// Invokes the event, calling all hooked callbacks.
    pub fn invoke(&self, args: &Args) {
        let inner = self.inner.lock().unwrap();
        for callback in &inner.callbacks {
            unsafe {
                (*callback.get())(args);
            }
        }
    }
}

#[derive(Debug)]
struct EventHookData<Args> {
    inner: Weak<Mutex<EventInner<Args>>>,
    callback: Weak<UnsafeCell<dyn FnMut(&Args)>>,
}

/// A hook into an event.
///
/// Hooks can be dropped to remove it from the event or
/// leaked to keep it alive till the event is dropped.
#[derive(Debug)]
pub struct Hook<Args> {
    data: Option<EventHookData<Args>>,
}

impl<Args> Hook<Args> {
    /// Returns true if the event is still alive.
    pub fn is_alive(&self) -> bool {
        self.data
            .as_ref()
            .map(|data| data.inner.strong_count() > 0)
            .unwrap_or(false)
    }

    /// Leaks the hook, preventing it from being dropped.
    /// This is useful if you want to keep the hook around till the event is dropped.
    pub fn leak(mut self) {
        self.data.take();
    }
}

impl<Args> Leak for Hook<Args> {
    fn leak(mut self) {
        self.data.take();
    }
}

impl<Args> Drop for Hook<Args> {
    fn drop(&mut self) {
        self.data.as_ref().map(
            |EventHookData {
                 inner,
                 callback: handler,
             }| {
                inner.upgrade().map(|inner| {
                    let mut inner = inner.lock().unwrap();
                    handler.upgrade().map(|handler| {
                        inner.callbacks.retain(|h| !Arc::ptr_eq(h, &handler));
                    });
                });
            },
        );
    }
}
