//! ## vakthund-core::alloc::arena
//! **Arena allocators using `bumpalo`**
//!
//! This module provides arena-based memory allocation using the `bumpalo` crate.
//! Arena allocators are efficient for allocating many objects with a limited lifetime,
//! where you can deallocate the entire arena at once.

use bumpalo::Bump;

/// An arena allocator based on `bumpalo::Bump`.
pub struct ArenaAllocator {
    bump_allocator: Bump,
}

impl ArenaAllocator {
    /// Creates a new arena allocator.
    pub fn new() -> Self {
        ArenaAllocator {
            bump_allocator: Bump::new(),
        }
    }

    /// Allocates memory in the arena and returns a mutable reference to it.
    pub fn allocate<T>(&self, value: T) -> &mut T {
        self.bump_allocator.alloc(value)
    }

    /// Allocates memory for a value of type `T` but does not initialize it.
    /// Returns a mutable pointer to the uninitialized memory.
    pub fn allocate_uninit<T>(&self) -> *mut T {
        let ptr = self
            .bump_allocator
            .alloc_layout(std::alloc::Layout::new::<T>());
        ptr.as_ptr() as *mut T
    }
    /// Resets the arena, deallocating all allocations made within it.
    /// This is a very fast way to deallocate all memory in the arena at once.
    pub fn reset(&mut self) {
        self.bump_allocator.reset();
    }

    // You could add methods for more advanced arena operations if needed,
    // like custom allocation sizes, etc.
}

impl Default for ArenaAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_allocator_allocate() {
        let arena = ArenaAllocator::new();
        let value1 = arena.allocate(123u32);
        let value2 = arena.allocate(456u64);

        assert_eq!(*value1, 123);
        assert_eq!(*value2, 456);
    }

    #[test]
    fn test_arena_allocator_allocate_uninit() {
        let arena = ArenaAllocator::new();
        let ptr = arena.allocate_uninit::<u32>();
        // Safety: We are initializing the memory we just allocated.
        unsafe {
            *ptr = 789;
            assert_eq!(*ptr, 789);
        }
    }

    #[test]
    fn test_arena_allocator_reset() {
        let mut arena = ArenaAllocator::new();
        {
            // Limit scope of value1 and allocation
            let value1 = arena.allocate(111u32);
            arena.allocate(222u32);
            assert_eq!(*value1, 111); // Assert while value1 is still in scope
        } // value1 goes out of scope here, releasing the borrow
        {
            arena.reset();
            let value3 = arena.allocate(333u32); // Allocate after reset
            assert_eq!(*value3, 333); // New allocation works after reset
        }
    }
}
