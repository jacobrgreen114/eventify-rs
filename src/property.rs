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
use std::sync::*;

#[derive(Debug)]
struct PropertyData<T> {
    value: T,
    callbacks: Vec<Arc<UnsafeCell<dyn FnMut(&T) -> ()>>>,
}

impl<T> PropertyData<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            callbacks: Default::default(),
        }
    }

    fn invoke_all_callbacks(&mut self) {
        for callback in &mut self.callbacks {
            unsafe {
                (*callback.get())(&self.value);
            }
        }
    }

    fn invoke_excluding(&mut self, excluded: &Arc<UnsafeCell<dyn FnMut(&T) -> ()>>) {
        for callback in &mut self.callbacks {
            if !Arc::ptr_eq(excluded, callback) {
                unsafe {
                    (*callback.get())(&self.value);
                }
            }
        }
    }

    fn add_callback(&mut self, callback: Arc<UnsafeCell<dyn FnMut(&T) -> ()>>) {
        self.callbacks.push(callback);
    }

    fn remove_callback(&mut self, callback: &Arc<UnsafeCell<dyn FnMut(&T) -> ()>>) {
        self.callbacks.retain(|c| !Arc::ptr_eq(callback, c));
    }
}

/// A thread-safe property that can be hooked into for changed events.
#[derive(Debug)]
pub struct Property<T> {
    inner: Arc<RwLock<PropertyData<T>>>,
}

impl<T> Property<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(PropertyData::new(value))),
        }
    }

    /// Locks the property and returns a guard that can be used to read the value.
    pub fn read(&self) -> LockResult<PropertyReadGuard<'_, T>> {
        map_lock_result(self.inner.read(), |inner| PropertyReadGuard::from(inner))
    }

    /// Locks the property and returns a guard that can be used to read and write the value.
    pub fn write(&self) -> LockResult<PropertyWriteGuard<T>> {
        map_lock_result(self.inner.write(), |inner| PropertyWriteGuard::from(inner))
    }

    #[must_use]
    #[inline(always)]
    pub fn bind(&self, f: impl FnMut(&T) + 'static) -> ReadonlyBinding<T> {
        self.bind_internal(box_callback(f))
    }

    #[must_use]
    #[inline(always)]
    pub fn bind_mut(&self, f: impl FnMut(&T) + 'static) -> ReadWriteBinding<T> {
        self.bind_mut_internal(box_callback(f))
    }

    fn bind_internal(&self, f: Arc<UnsafeCell<dyn FnMut(&T) -> ()>>) -> ReadonlyBinding<T> {
        self.inner.write().unwrap().add_callback(f.clone());
        ReadonlyBinding {
            inner: Some(BindingData {
                property: self.inner.clone(),
                callback: f,
            }),
        }
    }

    fn bind_mut_internal(&self, f: Arc<UnsafeCell<dyn FnMut(&T) -> ()>>) -> ReadWriteBinding<T> {
        self.inner.write().unwrap().add_callback(f.clone());
        ReadWriteBinding {
            inner: Some(BindingData {
                property: self.inner.clone(),
                callback: f,
            }),
        }
    }
}

#[inline(always)]
fn map_lock_result<T, U, F>(result: LockResult<T>, f: F) -> LockResult<U>
where
    F: Fn(T) -> U,
{
    result
        .map(&f)
        .map_err(|e| PoisonError::new(f(e.into_inner())))
}

/// A guard that can be used to read the value of a property.
///
/// This does not invoke the changed event when dropped.
pub struct PropertyReadGuard<'a, T> {
    inner: RwLockReadGuard<'a, PropertyData<T>>,
}

impl<'a, T> PropertyReadGuard<'a, T> {
    pub fn get(&self) -> &T {
        &self.inner.value
    }
}

impl<'a, T> From<RwLockReadGuard<'a, PropertyData<T>>> for PropertyReadGuard<'a, T> {
    fn from(inner: RwLockReadGuard<'a, PropertyData<T>>) -> Self {
        Self { inner }
    }
}

impl<'a, T> From<RwLockWriteGuard<'a, PropertyData<T>>> for PropertyWriteGuard<'a, T> {
    fn from(inner: RwLockWriteGuard<'a, PropertyData<T>>) -> Self {
        Self { inner }
    }
}

/// A guard that can be used to read and write a property.
///
/// When dropped, the property's changed event is invoked.
pub struct PropertyWriteGuard<'a, T> {
    inner: RwLockWriteGuard<'a, PropertyData<T>>,
}

impl<'a, T> PropertyWriteGuard<'a, T> {
    pub fn get(&self) -> &T {
        &self.inner.value
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner.value
    }
}

impl<T> Drop for PropertyWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.inner.invoke_all_callbacks();
    }
}

fn box_callback<T>(f: impl FnMut(&T) + 'static) -> Arc<UnsafeCell<dyn FnMut(&T) -> ()>> {
    unimplemented!()
}

impl<T> Default for Property<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(Default::default())
    }
}

struct BindingData<T> {
    property: Arc<RwLock<PropertyData<T>>>,
    callback: Arc<UnsafeCell<dyn FnMut(&T) -> ()>>,
}

impl<T> BindingData<T> {
    fn unbind(&self) {
        self.property
            .write()
            .unwrap()
            .remove_callback(&self.callback);
    }
}

/// A readonly binding to a property.
#[derive(Debug)]
pub struct ReadonlyBinding<T> {
    inner: Option<BindingData<T>>,
}

impl<T> ReadonlyBinding<T> {
    fn leak(mut self) {
        self.inner.take();
    }

    fn read(&self) -> LockResult<BindingReadGuard<'_, T>> {
        map_lock_result(self.inner.as_ref().unwrap().property.read(), |inner| {
            BindingReadGuard::from(inner)
        })
    }
}

impl Drop for ReadonlyBinding<()> {
    fn drop(&mut self) {
        self.inner.as_ref().map(|data| {
            data.unbind();
        });
    }
}

/// A read-write binding to a property.
#[derive(Debug)]
pub struct ReadWriteBinding<T> {
    inner: Option<BindingData<T>>,
}

impl<T> ReadWriteBinding<T> {
    fn leak(mut self) {
        self.inner.take();
    }

    fn read(&self) -> LockResult<BindingReadGuard<'_, T>> {
        map_lock_result(self.inner.as_ref().unwrap().property.read(), |inner| {
            BindingReadGuard::from(inner)
        })
    }

    fn write(&self) -> LockResult<BindingWriteGuard<T>> {
        map_lock_result(self.inner.as_ref().unwrap().property.write(), |inner| {
            BindingWriteGuard::new(inner, self.inner.as_ref().unwrap())
        })
    }
}

impl Drop for ReadWriteBinding<()> {
    fn drop(&mut self) {
        self.inner.as_ref().map(|data| {
            data.unbind();
        });
    }
}

/// A guard that can be used to read the value of a property.
///
/// This does not invoke the changed event when dropped.
pub struct BindingReadGuard<'a, T> {
    inner: RwLockReadGuard<'a, PropertyData<T>>,
}

impl<'a, T> BindingReadGuard<'a, T> {
    pub fn get(&self) -> &T {
        &self.inner.value
    }
}

impl<'a, T> From<RwLockReadGuard<'a, PropertyData<T>>> for BindingReadGuard<'a, T> {
    fn from(inner: RwLockReadGuard<'a, PropertyData<T>>) -> Self {
        Self { inner }
    }
}

/// A guard that can be used to read and write a property.
///
/// When dropped, the property's changed event is invoked for every binding except itself.
pub struct BindingWriteGuard<'a, T> {
    inner: RwLockWriteGuard<'a, PropertyData<T>>,
    data: &'a BindingData<T>,
}

impl<'a, T> BindingWriteGuard<'a, T> {
    fn new(inner: RwLockWriteGuard<'a, PropertyData<T>>, data: &'a BindingData<T>) -> Self {
        Self { inner, data }
    }

    pub fn get(&self) -> &() {
        &self.inner.value
    }

    pub fn get_mut(&mut self) -> &mut () {
        &mut self.inner.value
    }
}

impl<T> Drop for BindingWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.inner.invoke_excluding(&self.data.callback);
    }
}
