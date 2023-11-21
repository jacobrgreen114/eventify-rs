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

use std::sync::{Arc, Mutex, RwLock};
use super::event::*;

#[derive(Debug)]
struct PropertyData<T> {
    value: T,
    changed_event: Event<T>,
}

#[derive(Debug)]
pub struct Property<T> {
    inner: Arc<RwLock<PropertyData<T>>>,
}

impl<T> Property<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(PropertyData {
                value,
                changed_event: Event::new(),
            })),
        }
    }

    #[must_use]
    pub fn bind_changed(&self, f: impl FnMut(&T) + 'static) -> PropertyBinding<T> {
        let inner = self.inner.write().unwrap();
        let hook = inner.changed_event.add_handler(f);

        PropertyBinding {
            data: Some(PropertyBindingData {
                inner: self.inner.clone(),
                hook,
            }),
        }
    }
}

impl<T> Default for Property<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(Default::default())
    }
}

#[derive(Debug)]
struct PropertyBindingData<T> {
    inner: Arc<RwLock<PropertyData<T>>>,
    hook: EventHook<T>,
}

#[derive(Debug)]
pub struct PropertyBinding<T> {
    data: Option<PropertyBindingData<T>>,
}

impl<T> PropertyBinding<T> {
    fn leak(mut self) {
        unsafe { self.data.unwrap_unchecked() }.hook.leak();
    }
}
