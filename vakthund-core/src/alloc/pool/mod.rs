//! ## vakthund-core::alloc::pool
//! **Fixed-size memory pools**
//!
//! This module implements fixed-size memory pools for efficient allocation
//! and deallocation of objects of the same size.
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub struct MemoryPool<T> {
    chunk_size: usize,
    chunks: Mutex<Vec<Box<[MaybeUninit<T>]>>>,
    free_indices: Mutex<Vec<usize>>,
    allocated_count: AtomicUsize,
    capacity: usize,
}

impl<T> MemoryPool<T> {
    pub fn new(chunk_size: usize, capacity: usize) -> Self {
        assert!(chunk_size > 0, "Chunk size must be greater than zero");
        assert!(capacity > 0, "Capacity must be greater than zero");

        println!(
            "MemoryPool::new: chunk_size={}, capacity={}",
            chunk_size, capacity
        ); // Debug Print

        let num_chunks = (capacity + chunk_size - 1) / chunk_size;
        println!("MemoryPool::new: num_chunks={}", num_chunks); // Debug Print
        let mut chunks = Vec::with_capacity(num_chunks);
        let mut free_indices = Vec::with_capacity(capacity);

        println!(
            "MemoryPool::new: Initial chunks.capacity()={}, free_indices.capacity()={}",
            chunks.capacity(),
            free_indices.capacity()
        ); // Debug Print

        for _ in 0..num_chunks {
            let mut vec = Vec::with_capacity(chunk_size);
            vec.resize_with(chunk_size, || MaybeUninit::uninit());
            chunks.push(vec.into_boxed_slice());
        }
        println!("MemoryPool::new: After chunk resize, chunks.len()={}, chunks[0].len() (if chunks not empty)={}", chunks.len(), chunks.get(0).map_or(0, |c| c.len())); // Debug Print

        for i in 0..capacity {
            free_indices.push(i);
        }
        println!(
            "MemoryPool::new: After free_indices push, free_indices.len()={}",
            free_indices.len()
        ); // Debug Print

        Self {
            chunk_size,
            chunks: Mutex::new(chunks),
            free_indices: Mutex::new(free_indices),
            allocated_count: AtomicUsize::new(0),
            capacity,
        }
    }

    /// Allocates an object from the memory pool.
    /// Returns `None` if the pool is full.
    pub fn allocate(&self) -> Option<PoolPtr<T>> {
        let mut free_indices_lock = self.free_indices.lock().unwrap();
        if let Some(index) = free_indices_lock.pop() {
            self.allocated_count.fetch_add(1, Ordering::Relaxed);
            Some(PoolPtr::new(self, index))
        } else {
            None // Pool is full
        }
    }

    /// Deallocates an object back to the memory pool.
    ///
    /// # Safety
    ///
    /// The `PoolPtr` must be valid and associated with this `MemoryPool`.
    pub unsafe fn deallocate(&self, ptr: PoolPtr<T>) {
        let index = ptr.index;
        // Simplified lock acquisition and usage:
        self.free_indices.lock().unwrap().push(index);
        self.allocated_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Returns the current number of allocated objects in the pool.
    pub fn allocated_count(&self) -> usize {
        self.allocated_count.load(Ordering::Relaxed)
    }

    /// Returns the total capacity of the memory pool.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the chunk size used by the memory pool.
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    // Helper function to get a mutable reference to the memory location for a given index
    #[inline]
    fn get_memory_location_mut(&self, index: usize) -> *mut T {
        let chunk_index = index / self.chunk_size;
        let offset_in_chunk = index % self.chunk_size;
        let mut chunks_lock = self.chunks.lock().unwrap();
        let chunk = &mut chunks_lock[chunk_index];
        chunk[offset_in_chunk].as_mut_ptr() as *mut T // Cast MaybeUninit<T>* to T*
    }
}

/// A pointer to an object allocated from a `MemoryPool`.
pub struct PoolPtr<'pool, T> {
    pool: &'pool MemoryPool<T>,
    index: usize,
    _phantom: std::marker::PhantomData<T>, // For variance and drop check
}

impl<'pool, T> PoolPtr<'pool, T> {
    #[inline]
    fn new(pool: &'pool MemoryPool<T>, index: usize) -> Self {
        Self {
            pool,
            index,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Returns a mutable reference to the allocated object.
    ///
    /// # Safety
    ///
    /// The caller must ensure that there are no other mutable references to the same object
    /// alive at the same time to prevent data races.
    #[inline]
    pub unsafe fn as_mut_ptr(&self) -> *mut T {
        self.pool.get_memory_location_mut(self.index)
    }

    /// Initializes the memory location pointed to by this `PoolPtr` with the given value.
    ///
    /// # Safety
    ///
    /// The memory location must be valid and uninitialized.
    #[inline]
    pub unsafe fn write(&self, value: T) {
        ptr::write(self.as_mut_ptr(), value);
    }

    /// Reads the value from the memory location pointed to by this `PoolPtr`.
    ///
    /// # Safety
    ///
    /// The memory location must be initialized and contain a valid value of type `T`.
    #[inline]
    pub unsafe fn read(&self) -> T {
        ptr::read(self.as_mut_ptr())
    }
}

impl<'pool, T> Drop for PoolPtr<'pool, T> {
    fn drop(&mut self) {
        // Directly deallocate using the pool's internals
        self.pool.free_indices.lock().unwrap().push(self.index);
        self.pool.allocated_count.fetch_sub(1, Ordering::Relaxed);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_allocate_deallocate() {
        let pool: MemoryPool<u32> = MemoryPool::new(10, 20);
        let _ptr1 = pool.allocate().unwrap();
    }

    #[test]
    fn test_memory_pool_capacity() {
        let pool: MemoryPool<u32> = MemoryPool::new(5, 10);
        let mut allocations = Vec::with_capacity(10);

        for _ in 0..10 {
            allocations.push(pool.allocate().unwrap());
        }

        // At this point, all 10 allocations are alive.
        assert_eq!(pool.allocated_count(), 10);
        assert!(pool.allocate().is_none()); // Pool is full

        // allocations are dropped here, triggering deallocation.
    }

    #[test]
    #[should_panic]
    fn test_memory_pool_zero_chunk_size() {
        MemoryPool::<u32>::new(0, 10);
    }

    #[test]
    #[should_panic]
    fn test_memory_pool_zero_capacity() {
        MemoryPool::<u32>::new(10, 0);
    }
}
