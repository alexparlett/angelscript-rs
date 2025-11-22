//! Garbage Collector for AngelScript-in-Rust
//!
//! This implements AngelScript's hybrid memory management approach:
//! - Reference counting is the primary mechanism
//! - GC is backup only for circular references
//!
//! ## Architecture
//!
//! The GC tracks objects minimally - it only stores:
//! - Object handle
//! - Type ID (for looking up behaviours)
//! - References held by the object
//! - Generation tracking
//!
//! All ref counting and GC flag operations are done via `call_system_function()`
//! which calls the appropriate behaviours (GetRefCount, SetGCFlag, etc.).
//!
//! ## Algorithm (from AngelScript documentation)
//! 1. Destroy garbage: Free objects with only 1 reference (held by GC)
//! 2. Clear counters: Reset GC counters and set flags
//! 3. Count GC references: Count references between GC objects
//! 4. Mark live objects: Objects where gc_count != refcount are live
//! 5. Verify unmarked: Re-check if unmarked objects got external refs
//! 6. Break circular references: Call ReleaseRefs on dead objects

use crate::core::types::TypeId;
use std::collections::{HashMap, HashSet, VecDeque};

/// GC operation flags (matching AngelScript's asEGCFlags)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GCFlags(u32);

impl GCFlags {
    /// Execute one incremental step
    pub const ONE_STEP: GCFlags = GCFlags(0x01);
    /// Execute a full cycle
    pub const FULL_CYCLE: GCFlags = GCFlags(0x02);
    /// Only destroy trivial garbage (refcount == 1)
    pub const DESTROY_GARBAGE: GCFlags = GCFlags(0x04);
    /// Detect circular references
    pub const DETECT_GARBAGE: GCFlags = GCFlags(0x08);

    pub fn contains(&self, other: GCFlags) -> bool {
        (self.0 & other.0) != 0
    }

    pub fn from_bits(bits: u32) -> Self {
        GCFlags(bits)
    }
}

impl std::ops::BitOr for GCFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        GCFlags(self.0 | rhs.0)
    }
}

/// Statistics from the garbage collector
#[derive(Debug, Clone, Default)]
pub struct GCStatistics {
    /// Current number of objects tracked by GC
    pub current_size: u32,
    /// Total objects destroyed
    pub total_destroyed: u64,
    /// Objects destroyed due to circular references
    pub total_detected_as_garbage: u64,
    /// Number of objects in new generation
    pub new_objects: u32,
    /// Number of objects in old generation
    pub old_objects: u32,
}

/// State of the incremental GC algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GCState {
    /// Ready to start a new cycle
    Idle,
    /// Step 1: Destroying trivial garbage
    DestroyGarbage,
    /// Step 2: Clearing counters and setting flags
    ClearCounters,
    /// Step 3: Counting GC references
    CountReferences,
    /// Step 4: Marking live objects
    MarkLiveObjects,
    /// Step 5: Verifying unmarked objects
    VerifyUnmarked,
    /// Step 6: Breaking circular references
    BreakCircularRefs,
}

/// Minimal tracking info for a GC-managed object
///
/// The actual ref_count and gc_flag are stored on the objects themselves
/// (ScriptObject or application types). The GC accesses them via behaviours.
#[derive(Debug)]
struct GCEntry {
    /// The object handle
    object_id: u64,
    /// Type ID for looking up behaviours
    type_id: TypeId,
    /// Cached reference count (updated during GC phases)
    cached_ref_count: u32,
    /// Cached GC flag (updated during GC phases)
    cached_gc_flag: bool,
    /// GC counter - counts references from other GC objects
    gc_count: u32,
    /// Which generation (true = old, false = new)
    is_old_generation: bool,
    /// Number of iterations survived in new generation
    survival_count: u32,
    /// References this object holds to other GC objects
    held_references: Vec<u64>,
}

impl GCEntry {
    fn new(object_id: u64, type_id: TypeId) -> Self {
        Self {
            object_id,
            type_id,
            cached_ref_count: 1, // Assume initial ref from GC
            cached_gc_flag: false,
            gc_count: 0,
            is_old_generation: false,
            survival_count: 0,
            held_references: Vec::new(),
        }
    }
}

/// The Garbage Collector
///
/// Implements AngelScript's incremental GC algorithm for detecting
/// and breaking circular references.
///
/// The GC is type-agnostic - it tracks objects by handle and calls
/// behaviours via `call_system_function()` to get ref counts, set flags, etc.
pub struct GarbageCollector {
    /// New generation objects (recently allocated)
    new_generation: HashMap<u64, GCEntry>,
    /// Old generation objects (survived multiple GC cycles)
    old_generation: HashMap<u64, GCEntry>,

    /// Current state of the GC algorithm
    state: GCState,
    /// Iterator position for incremental processing
    iterator_position: usize,
    /// Objects marked as potentially alive
    live_objects: HashSet<u64>,
    /// Objects to be destroyed
    garbage_objects: Vec<u64>,
    /// Work queue for current step
    work_queue: VecDeque<u64>,

    /// Statistics
    stats: GCStatistics,

    /// Number of iterations before promoting to old generation
    promotion_threshold: u32,
    /// Auto GC enabled
    auto_gc_enabled: bool,
    /// Number of allocations since last auto GC
    allocations_since_gc: u32,
    /// Threshold for triggering auto GC
    auto_gc_threshold: u32,
}

impl GarbageCollector {
    pub fn new() -> Self {
        Self {
            new_generation: HashMap::new(),
            old_generation: HashMap::new(),
            state: GCState::Idle,
            iterator_position: 0,
            live_objects: HashSet::new(),
            garbage_objects: Vec::new(),
            work_queue: VecDeque::new(),
            stats: GCStatistics::default(),
            promotion_threshold: 3,
            auto_gc_enabled: true,
            allocations_since_gc: 0,
            auto_gc_threshold: 100,
        }
    }

    /// Register a new GC object (called when GC-enabled object is created)
    ///
    /// This is equivalent to NotifyGarbageCollectorOfNewObject() in AngelScript.
    pub fn add_object(&mut self, object_id: u64, type_id: TypeId) {
        let entry = GCEntry::new(object_id, type_id);
        self.new_generation.insert(object_id, entry);
        self.stats.current_size += 1;
        self.stats.new_objects += 1;

        // Trigger auto GC if enabled
        if self.auto_gc_enabled {
            self.allocations_since_gc += 1;
            if self.allocations_since_gc >= self.auto_gc_threshold {
                // Run a few incremental steps
                for _ in 0..3 {
                    self.garbage_collect(GCFlags::ONE_STEP);
                }
                self.allocations_since_gc = 0;
            }
        }
    }

    /// Remove an object from GC tracking (called when object is destroyed)
    pub fn remove_object(&mut self, object_id: u64) {
        if self.new_generation.remove(&object_id).is_some() {
            self.stats.new_objects = self.stats.new_objects.saturating_sub(1);
        } else if self.old_generation.remove(&object_id).is_some() {
            self.stats.old_objects = self.stats.old_objects.saturating_sub(1);
        }
        self.stats.current_size = self.stats.current_size.saturating_sub(1);
    }

    /// Get the type ID for a tracked object
    pub fn get_type_id(&self, object_id: u64) -> Option<TypeId> {
        self.new_generation
            .get(&object_id)
            .or_else(|| self.old_generation.get(&object_id))
            .map(|e| e.type_id)
    }

    /// Update the cached reference count for an object
    ///
    /// Call this after calling GetRefCount behaviour via call_system_function()
    pub fn update_ref_count(&mut self, object_id: u64, ref_count: u32) {
        if let Some(entry) = self.new_generation.get_mut(&object_id) {
            entry.cached_ref_count = ref_count;
        } else if let Some(entry) = self.old_generation.get_mut(&object_id) {
            entry.cached_ref_count = ref_count;
        }
    }

    /// Update the cached GC flag for an object
    ///
    /// Call this after calling GetGCFlag behaviour via call_system_function()
    pub fn update_gc_flag(&mut self, object_id: u64, gc_flag: bool) {
        if let Some(entry) = self.new_generation.get_mut(&object_id) {
            entry.cached_gc_flag = gc_flag;
        } else if let Some(entry) = self.old_generation.get_mut(&object_id) {
            entry.cached_gc_flag = gc_flag;
        }
    }

    /// Update the references held by an object
    ///
    /// Call this after calling EnumRefs behaviour via call_system_function()
    /// or after reading references from a ScriptObject directly.
    pub fn set_object_references(&mut self, object_id: u64, references: Vec<u64>) {
        if let Some(entry) = self.new_generation.get_mut(&object_id) {
            entry.held_references = references;
        } else if let Some(entry) = self.old_generation.get_mut(&object_id) {
            entry.held_references = references;
        }
    }

    /// Get cached reference count
    pub fn get_cached_ref_count(&self, object_id: u64) -> Option<u32> {
        self.new_generation
            .get(&object_id)
            .or_else(|| self.old_generation.get(&object_id))
            .map(|e| e.cached_ref_count)
    }

    /// Get cached GC flag
    pub fn get_cached_gc_flag(&self, object_id: u64) -> Option<bool> {
        self.new_generation
            .get(&object_id)
            .or_else(|| self.old_generation.get(&object_id))
            .map(|e| e.cached_gc_flag)
    }

    /// Get statistics
    pub fn get_statistics(&self) -> GCStatistics {
        GCStatistics {
            current_size: self.stats.current_size,
            total_destroyed: self.stats.total_destroyed,
            total_detected_as_garbage: self.stats.total_detected_as_garbage,
            new_objects: self.new_generation.len() as u32,
            old_objects: self.old_generation.len() as u32,
        }
    }

    /// Enable/disable automatic garbage collection
    pub fn set_auto_gc(&mut self, enabled: bool) {
        self.auto_gc_enabled = enabled;
    }

    /// Check if an object is tracked by the GC
    pub fn is_tracked(&self, object_id: u64) -> bool {
        self.new_generation.contains_key(&object_id)
            || self.old_generation.contains_key(&object_id)
    }

    /// Get all object IDs tracked by the GC
    pub fn get_all_object_ids(&self) -> Vec<u64> {
        self.new_generation
            .keys()
            .chain(self.old_generation.keys())
            .copied()
            .collect()
    }

    /// Get all object IDs that need behaviour calls
    ///
    /// Returns (object_id, type_id) pairs for all tracked objects.
    /// Use this to call behaviours via call_system_function().
    pub fn get_all_objects_with_types(&self) -> Vec<(u64, TypeId)> {
        self.new_generation
            .values()
            .chain(self.old_generation.values())
            .map(|e| (e.object_id, e.type_id))
            .collect()
    }

    /// Main GC entry point
    ///
    /// flags can be:
    /// - ONE_STEP: Execute one incremental step
    /// - FULL_CYCLE: Execute until complete
    /// - DESTROY_GARBAGE: Only destroy trivial garbage
    /// - DETECT_GARBAGE: Also detect circular references
    ///
    /// Returns number of objects processed/destroyed.
    pub fn garbage_collect(&mut self, flags: GCFlags) -> u32 {
        let mut work_done = 0;

        if flags.contains(GCFlags::FULL_CYCLE) {
            // Run complete cycle
            loop {
                let step_work = self.step(flags);
                work_done += step_work;
                if self.state == GCState::Idle {
                    break;
                }
            }
        } else if flags.contains(GCFlags::ONE_STEP) {
            // Run one step
            work_done = self.step(flags);
        }

        work_done
    }

    /// Execute one step of the GC algorithm
    fn step(&mut self, flags: GCFlags) -> u32 {
        match self.state {
            GCState::Idle => {
                self.state = GCState::DestroyGarbage;
                self.iterator_position = 0;
                1
            }

            GCState::DestroyGarbage => self.step_destroy_garbage(),

            GCState::ClearCounters => {
                if flags.contains(GCFlags::DESTROY_GARBAGE)
                    && !flags.contains(GCFlags::DETECT_GARBAGE)
                {
                    // Only destroy garbage, don't detect circular refs
                    self.state = GCState::Idle;
                    return 1;
                }
                self.step_clear_counters()
            }

            GCState::CountReferences => self.step_count_references(),

            GCState::MarkLiveObjects => self.step_mark_live_objects(),

            GCState::VerifyUnmarked => self.step_verify_unmarked(),

            GCState::BreakCircularRefs => self.step_break_circular_refs(),
        }
    }

    /// Step 1: Destroy trivial garbage (cached_ref_count == 1, only GC reference)
    fn step_destroy_garbage(&mut self) -> u32 {
        let mut destroyed = 0;

        // Process new generation first (more likely to have trivial garbage)
        let trivial_new: Vec<u64> = self
            .new_generation
            .iter()
            .filter(|(_, e)| e.cached_ref_count == 1)
            .map(|(id, _)| *id)
            .collect();

        for object_id in trivial_new {
            self.new_generation.remove(&object_id);
            self.stats.new_objects = self.stats.new_objects.saturating_sub(1);
            self.stats.current_size = self.stats.current_size.saturating_sub(1);
            self.stats.total_destroyed += 1;
            destroyed += 1;
        }

        // Process old generation
        let trivial_old: Vec<u64> = self
            .old_generation
            .iter()
            .filter(|(_, e)| e.cached_ref_count == 1)
            .map(|(id, _)| *id)
            .collect();

        for object_id in trivial_old {
            self.old_generation.remove(&object_id);
            self.stats.old_objects = self.stats.old_objects.saturating_sub(1);
            self.stats.current_size = self.stats.current_size.saturating_sub(1);
            self.stats.total_destroyed += 1;
            destroyed += 1;
        }

        // Promote survivors
        self.promote_survivors();

        self.state = GCState::ClearCounters;
        destroyed.max(1)
    }

    /// Promote new generation objects that have survived enough cycles
    fn promote_survivors(&mut self) {
        let to_promote: Vec<u64> = self
            .new_generation
            .iter()
            .filter(|(_, e)| e.survival_count >= self.promotion_threshold)
            .map(|(id, _)| *id)
            .collect();

        for object_id in to_promote {
            if let Some(mut entry) = self.new_generation.remove(&object_id) {
                entry.is_old_generation = true;
                self.old_generation.insert(object_id, entry);
                self.stats.new_objects = self.stats.new_objects.saturating_sub(1);
                self.stats.old_objects += 1;
            }
        }

        // Increment survival count for remaining new objects
        for entry in self.new_generation.values_mut() {
            entry.survival_count += 1;
        }
    }

    /// Step 2: Clear GC counters and set flags
    fn step_clear_counters(&mut self) -> u32 {
        // Only process old generation for cycle detection
        for entry in self.old_generation.values_mut() {
            entry.gc_count = 0;
            entry.cached_gc_flag = true; // Will be set via behaviour call
        }

        self.live_objects.clear();
        self.garbage_objects.clear();
        self.state = GCState::CountReferences;
        1
    }

    /// Step 3: Count references from GC objects to other GC objects
    fn step_count_references(&mut self) -> u32 {
        // For each object that still has its flag set (not externally referenced),
        // count references to other GC objects
        let flagged_refs: Vec<Vec<u64>> = self
            .old_generation
            .values()
            .filter(|e| e.cached_gc_flag)
            .map(|e| e.held_references.clone())
            .collect();

        // Increment gc_count for referenced objects
        for refs in flagged_refs {
            for ref_id in refs {
                if let Some(target) = self.old_generation.get_mut(&ref_id) {
                    target.gc_count += 1;
                }
            }
        }

        self.state = GCState::MarkLiveObjects;
        1
    }

    /// Step 4: Mark objects as live
    ///
    /// An object is live if:
    /// - Its GC flag is not set (externally referenced)
    /// - Its gc_count != cached_ref_count (external references exist)
    fn step_mark_live_objects(&mut self) -> u32 {
        self.work_queue.clear();

        for (object_id, entry) in &self.old_generation {
            // If flag is cleared, object was externally accessed during GC
            if !entry.cached_gc_flag {
                self.live_objects.insert(*object_id);
                self.work_queue.push_back(*object_id);
            }
            // If gc_count != cached_ref_count, there are external references
            else if entry.gc_count != entry.cached_ref_count {
                self.live_objects.insert(*object_id);
                self.work_queue.push_back(*object_id);
            }
        }

        // Propagate liveness through references
        while let Some(live_id) = self.work_queue.pop_front() {
            if let Some(entry) = self.old_generation.get(&live_id) {
                for ref_id in &entry.held_references {
                    if !self.live_objects.contains(ref_id)
                        && self.old_generation.contains_key(ref_id)
                    {
                        self.live_objects.insert(*ref_id);
                        self.work_queue.push_back(*ref_id);
                    }
                }
            }
        }

        self.state = GCState::VerifyUnmarked;
        1
    }

    /// Step 5: Verify unmarked objects
    ///
    /// Check if any unmarked objects had their flag cleared during marking
    fn step_verify_unmarked(&mut self) -> u32 {
        let mut need_remark = false;

        for (object_id, entry) in &self.old_generation {
            if !self.live_objects.contains(object_id) {
                // This object wasn't marked as live
                // Check if its flag was cleared (external access during GC)
                if !entry.cached_gc_flag {
                    // External access occurred - need to re-mark
                    self.live_objects.insert(*object_id);
                    need_remark = true;
                }
            }
        }

        if need_remark {
            // Go back to mark live objects to propagate the new live objects
            self.state = GCState::MarkLiveObjects;
        } else {
            // Collect garbage objects (not in live set)
            self.garbage_objects = self
                .old_generation
                .keys()
                .filter(|id| !self.live_objects.contains(id))
                .copied()
                .collect();

            self.state = GCState::BreakCircularRefs;
        }

        1
    }

    /// Step 6: Break circular references
    ///
    /// Objects not marked as live are involved in circular references.
    /// Returns handles of objects that should be destroyed.
    fn step_break_circular_refs(&mut self) -> u32 {
        let garbage_count = self.garbage_objects.len() as u32;

        if garbage_count > 0 {
            self.stats.total_detected_as_garbage += garbage_count as u64;

            // Clear references for all garbage objects (break cycles)
            for object_id in &self.garbage_objects {
                if let Some(entry) = self.old_generation.get_mut(object_id) {
                    entry.held_references.clear();
                }
            }

            // Remove garbage objects from tracking
            for object_id in &self.garbage_objects {
                self.old_generation.remove(object_id);
                self.stats.old_objects = self.stats.old_objects.saturating_sub(1);
                self.stats.current_size = self.stats.current_size.saturating_sub(1);
                self.stats.total_destroyed += 1;
            }

            self.garbage_objects.clear();
        }

        self.state = GCState::Idle;
        garbage_count.max(1)
    }

    /// Get objects that need to be destroyed
    ///
    /// After running garbage_collect(), call this to get the list of
    /// objects that should have their Release behaviour called and
    /// then be removed from the heap.
    pub fn get_garbage_to_destroy(&self) -> &[u64] {
        &self.garbage_objects
    }
}

impl Default for GarbageCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_track_object() {
        let mut gc = GarbageCollector::new();
        gc.set_auto_gc(false);
        gc.add_object(1, 100);

        assert!(gc.is_tracked(1));
        assert_eq!(gc.get_statistics().current_size, 1);
        assert_eq!(gc.get_statistics().new_objects, 1);
    }

    #[test]
    fn test_trivial_garbage_collection() {
        let mut gc = GarbageCollector::new();
        gc.set_auto_gc(false);

        // Add an object with only GC reference (cached_ref_count = 1)
        gc.add_object(1, 100);

        // Object has refcount 1 (GC reference only) - should be collected
        gc.garbage_collect(GCFlags::FULL_CYCLE | GCFlags::DESTROY_GARBAGE);

        assert!(!gc.is_tracked(1));
        assert_eq!(gc.get_statistics().total_destroyed, 1);
    }

    #[test]
    fn test_external_reference_prevents_collection() {
        let mut gc = GarbageCollector::new();
        gc.set_auto_gc(false);

        gc.add_object(1, 100);

        // Simulate external reference by updating cached ref count
        gc.update_ref_count(1, 2);

        // Object now has refcount 2 - should NOT be collected
        gc.garbage_collect(GCFlags::FULL_CYCLE | GCFlags::DESTROY_GARBAGE);

        assert!(gc.is_tracked(1));
    }

    #[test]
    fn test_statistics() {
        let mut gc = GarbageCollector::new();
        gc.set_auto_gc(false);

        gc.add_object(1, 100);
        gc.add_object(2, 100);
        gc.add_object(3, 100);

        let stats = gc.get_statistics();
        assert_eq!(stats.current_size, 3);
        assert_eq!(stats.new_objects, 3);
        assert_eq!(stats.old_objects, 0);

        // Collect trivial garbage
        gc.garbage_collect(GCFlags::FULL_CYCLE | GCFlags::DESTROY_GARBAGE);

        let stats = gc.get_statistics();
        assert_eq!(stats.total_destroyed, 3);
    }

    #[test]
    fn test_update_gc_flag() {
        let mut gc = GarbageCollector::new();
        gc.set_auto_gc(false);

        gc.add_object(1, 100);
        assert_eq!(gc.get_cached_gc_flag(1), Some(false));

        gc.update_gc_flag(1, true);
        assert_eq!(gc.get_cached_gc_flag(1), Some(true));
    }

    #[test]
    fn test_set_object_references() {
        let mut gc = GarbageCollector::new();
        gc.set_auto_gc(false);

        gc.add_object(1, 100);
        gc.add_object(2, 100);

        gc.set_object_references(1, vec![2]);

        // Promote to old generation for testing
        gc.update_ref_count(1, 2);
        gc.update_ref_count(2, 2);
        for _ in 0..4 {
            gc.garbage_collect(GCFlags::FULL_CYCLE | GCFlags::DESTROY_GARBAGE);
        }

        // Both should still be tracked (external refs)
        assert!(gc.is_tracked(1));
        assert!(gc.is_tracked(2));
    }
}