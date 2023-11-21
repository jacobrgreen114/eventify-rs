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
use std::env::Args;
use std::sync::{Arc, Mutex, Weak};

#[derive(Debug, Default)]
struct EventInner<Args> {
    handlers: Vec<Arc<UnsafeCell<dyn FnMut(&Args) -> ()>>>,
}

#[derive(Debug, Default)]
pub struct Event<Args> {
    inner: Arc<Mutex<EventInner<Args>>>,
}

impl<Args> Event<Args> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(EventInner {
                handlers: Vec::new(),
            })),
        }
    }

    #[must_use]
    pub fn add_handler(&self, handler: impl FnMut(&Args) + 'static) -> EventHook<Args> {
        let mut inner = self.inner.lock().unwrap();
        let handler = Arc::new(UnsafeCell::new(handler));

        let weak_inner = Arc::downgrade(&self.inner);
        let weak_handler = Arc::downgrade(&handler);

        inner.handlers.push(handler);
        EventHook {
            data: Some(EventHookData {
                inner: weak_inner,
                handler: weak_handler,
            }),
        }
    }

    pub fn invoke(&self, args: &Args) {
        let inner = self.inner.lock().unwrap();
        for handler in &inner.handlers {
            unsafe {
                (*handler.get())(args);
            }
        }
    }
}

#[derive(Debug)]
struct EventHookData<Args> {
    inner: Weak<Mutex<EventInner<Args>>>,
    handler: Weak<UnsafeCell<dyn FnMut(&Args)>>,
}

#[derive(Debug)]
pub struct EventHook<Args> {
    data: Option<EventHookData<Args>>,
}

impl<Args> EventHook<Args> {
    pub fn leak(mut self) {
        self.data.take();
    }
}

impl<Args> Drop for EventHook<Args> {
    fn drop(&mut self) {
        self.data.as_ref().map(|EventHookData { inner, handler }| {
            inner.upgrade().map(|inner| {
                let mut inner = inner.lock().unwrap();
                handler.upgrade().map(|handler| {
                    inner.handlers.retain(|h| !Arc::ptr_eq(h, &handler));
                });
            });
        });
    }
}
