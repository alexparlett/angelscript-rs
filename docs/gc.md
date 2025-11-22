# Garbage Collection Design Document

## Overview

AngelScript uses a **hybrid memory management** approach:

1. **Reference Counting** - Primary mechanism, handles ~99% of object lifetimes
2. **Garbage Collection** - Backup mechanism, only for circular references

This design was chosen because reference counting is simple, deterministic, and allows easy interop between script and application code. The GC only exists to handle the edge case where circular references prevent refcounts from reaching zero.

## The Problem: Circular References

Reference counting works perfectly for linear ownership:

```
A → B → C
```

When A is released, B's refcount drops to 0, B is destroyed, C's refcount drops to 0, C is destroyed. Clean.

But circular references break this:

```angelscript
class Node {
    Node@ next;
    Node@ prev;
}

void createCycle() {
    Node@ a = Node();  // a.refcount = 1
    Node@ b = Node();  // b.refcount = 1
    
    @a.next = @b;      // b.refcount = 2
    @b.prev = @a;      // a.refcount = 2
    
    // End of scope:
    // - Release handle 'a': a.refcount = 2 → 1 (b.prev still holds ref)
    // - Release handle 'b': b.refcount = 2 → 1 (a.next still holds ref)
    // 
    // LEAK! Both objects have refcount=1, neither will ever reach 0
}
```

The GC exists solely to detect and break these cycles.

## GC Architecture

### What Gets Tracked

Not all types participate in GC. Only types registered with `TypeFlags::GC_TYPE` are tracked:

```rust
// GC-enabled type (can form circular references)
engine.register_object_type::<Node>("Node", TypeFlags::REF_TYPE | TypeFlags::GC_TYPE)?;

// Non-GC type (cannot form cycles, or app manages lifetime)
engine.register_object_type::<Vec3>("Vec3", TypeFlags::VALUE_TYPE)?;
```

**Rule of thumb**: If a type can hold a handle (`@`) to its own type (directly or indirectly), it should be GC-enabled.

### The GC's Reference

**Critical concept**: The GC holds ONE reference to every GC-tracked object.

When a GC object is created:
```
Object created → refcount = 1 (GC's reference)
Script gets handle → refcount = 2 (GC + script)
```

This means:
- `refcount == 1` → Only the GC holds a reference (trivial garbage)
- `refcount > 1` → External references exist

### Generational Collection

Objects are divided into two generations for efficiency:

**New Generation**
- Recently allocated objects
- Most objects die young (allocated, used briefly, released)
- Only trivial GC (destroy objects with refcount=1)
- Cheap to process

**Old Generation**
- Objects that survived multiple GC cycles
- More likely to be long-lived or in cycles
- Full cycle detection algorithm runs here
- More expensive to process

Objects are promoted from new → old after surviving N cycles (default: 3).

## The GC Algorithm

Based on AngelScript's incremental algorithm from `doc_memory.html`.

### Step 1: Destroy Trivial Garbage

```
For each object in GC:
    if object.refcount == 1:
        // Only GC holds a reference - nobody else wants this object
        destroy(object)
```

This is fast and handles most garbage without complex cycle detection.

### Step 2: Clear Counters and Set Flags

```
For each remaining object:
    object.gc_count = 0      // Will count refs from other GC objects
    object.gc_flag = true    // Mark as "not externally touched"
```

The `gc_flag` is crucial: it detects if external code (application/script) touches the object during GC.

### Step 3: Count GC-Internal References

```
For each object where gc_flag is still true:
    For each reference this object holds:
        if target is a GC object:
            target.gc_count += 1
```

After this step, `gc_count` represents how many references come from OTHER GC objects.

**Key insight**: If `gc_count == refcount`, ALL references to this object come from within the GC system. No external code can reach it.

### Step 4: Mark Live Objects

```
live_set = {}

For each object:
    if gc_flag == false:
        // External code touched this object during GC
        live_set.add(object)
    else if gc_count != refcount:
        // Some references come from outside GC
        live_set.add(object)

// Propagate liveness through references
worklist = live_set.copy()
while worklist not empty:
    obj = worklist.pop()
    for each reference obj holds:
        if reference not in live_set:
            live_set.add(reference)
            worklist.add(reference)
```

### Step 5: Verify Unmarked Objects

```
For each object not in live_set:
    if object.gc_flag == false:
        // External access occurred during marking!
        // Need to re-run step 4
        restart_marking()
```

This handles race conditions where external code accesses an object while GC is running.

### Step 6: Break Circular References

```
For each object not in live_set:
    // This object is unreachable - it's in a cycle
    object.release_all_references()  // Break the cycle
    
// After breaking cycles, objects' refcounts will drop
// Normal reference counting takes over and destroys them
```

## The GC Flag Mechanism

The `gc_flag` is essential for correctness. Here's how it works:

### Setting the Flag

The GC sets `gc_flag = true` at the start of cycle detection (Step 2).

### Clearing the Flag

**AddRef and Release automatically clear the flag**:

```rust
impl Object {
    pub fn add_ref(&self) {
        if self.gc_tracked {
            self.gc_flag = false;  // External activity detected!
        }
        self.refcount += 1;
    }
    
    pub fn release(&self) -> bool {
        if self.gc_tracked {
            self.gc_flag = false;  // External activity detected!
        }
        self.refcount -= 1;
        self.refcount == 0
    }
}
```

### Why This Works

If external code (script or application) accesses an object during GC:
1. It must go through AddRef/Release
2. This clears the gc_flag
3. GC sees the cleared flag and knows the object is live

This allows GC to run incrementally without stopping the world.

## GC Behaviors

GC-enabled types must support these behaviors:

### Required for Reference Types

| Behavior | Signature | Purpose |
|----------|-----------|---------|
| `AddRef` | `void f()` | Increment refcount, clear gc_flag |
| `Release` | `void f()` | Decrement refcount, clear gc_flag, destroy if 0 |
| `GetRefCount` | `int f()` | Return current refcount |
| `SetGCFlag` | `void f()` | Set the gc_flag to true |
| `GetGCFlag` | `bool f()` | Return current gc_flag value |
| `EnumRefs` | `void f(int&in)` | Enumerate all held references |
| `ReleaseRefs` | `void f(int&in)` | Release all held references |

### For Script Objects

Script objects (`ScriptObject`) have these behaviors built-in. The engine automatically:
- Tracks refcount
- Manages gc_flag
- Enumerates handle properties
- Releases handle properties

### For Application Objects

Application-registered types must implement these behaviors:

```rust
// Example: Registering a GC-enabled application type
engine.register_object_type::<Container>("Container", TypeFlags::REF_TYPE | TypeFlags::GC_TYPE)?;

engine.register_object_behaviour("Container", BehaviourType::AddRef, "void f()")?;
engine.register_object_behaviour("Container", BehaviourType::Release, "void f()")?;
engine.register_object_behaviour("Container", BehaviourType::GetRefCount, "int f()")?;
engine.register_object_behaviour("Container", BehaviourType::SetGCFlag, "void f()")?;
engine.register_object_behaviour("Container", BehaviourType::GetGCFlag, "bool f()")?;
engine.register_object_behaviour("Container", BehaviourType::EnumRefs, "void f(int&in)")?;
engine.register_object_behaviour("Container", BehaviourType::ReleaseRefs, "void f(int&in)")?;
```

**Note**: There is no compile-time validation that all behaviors are registered. Missing behaviors will cause runtime errors when the GC tries to use them.

## Running the GC

### Automatic GC

By default, the GC runs automatically:
- A few incremental steps after each GC object allocation
- Controlled by `auto_gc_enabled` flag and `auto_gc_threshold`

```rust
// Disable automatic GC for performance-critical sections
vm.set_auto_gc(false);

// Re-enable
vm.set_auto_gc(true);
```

### Manual GC

The application can trigger GC manually:

```rust
// Run one incremental step (non-blocking, for responsive apps)
vm.garbage_collect(GCFlags::ONE_STEP);

// Run full cycle (may pause, for idle time)
vm.garbage_collect(GCFlags::FULL_CYCLE);

// Only destroy trivial garbage (fast)
vm.garbage_collect(GCFlags::FULL_CYCLE | GCFlags::DESTROY_GARBAGE);

// Full cycle with circular reference detection
vm.garbage_collect(GCFlags::FULL_CYCLE | GCFlags::DESTROY_GARBAGE | GCFlags::DETECT_GARBAGE);
```

### GC Statistics

Monitor GC performance:

```rust
let stats = vm.get_gc_statistics();
println!("Objects tracked: {}", stats.current_size);
println!("New generation: {}", stats.new_objects);
println!("Old generation: {}", stats.old_objects);
println!("Total destroyed: {}", stats.total_destroyed);
println!("Cycles detected: {}", stats.total_detected_as_garbage);
```

## Implementation Components

### GCObject

Per-object tracking data:

```rust
pub struct GCObject {
    pub object_id: u64,           // Handle to actual object
    pub type_id: TypeId,          // Type information
    pub ref_count: AtomicU32,     // Reference count
    pub gc_flag: AtomicBool,      // Cleared by AddRef/Release
    pub gc_count: AtomicU32,      // References from other GC objects
    pub is_old_generation: bool,  // Which generation
    pub survival_count: u32,      // Cycles survived (for promotion)
    pub held_references: Vec<u64>, // References this object holds
}
```

### GarbageCollector

The main GC implementation:

```rust
pub struct GarbageCollector {
    new_generation: HashMap<u64, GCObject>,
    old_generation: HashMap<u64, GCObject>,
    state: GCState,  // Current step in incremental algorithm
    // ... statistics, configuration, work queues
}
```

### Integration with ObjectHeap

The heap integrates with GC:

```rust
impl ObjectHeap {
    pub fn allocate_script(&mut self, type_id: TypeId) -> Result<u64, String> {
        // Check if type has GC flag
        let is_gc_type = /* ... */;
        
        let object = if is_gc_type {
            Object::new_script_gc(type_id)
        } else {
            Object::new_script(type_id)
        };
        
        // Register with GC if needed
        if is_gc_type {
            self.garbage_collector.add_object(object_id, type_id);
        }
        
        Ok(object_id)
    }
}
```

## Example: Cycle Detection Walkthrough

Let's trace through detecting a simple cycle:

```angelscript
class Node { Node@ next; }

Node@ a = Node();  // a.refcount = 2 (GC + handle)
Node@ b = Node();  // b.refcount = 2 (GC + handle)
@a.next = @b;      // b.refcount = 3 (GC + handle + a.next)
@b.next = @a;      // a.refcount = 3 (GC + handle + b.next)

// Handles released at end of scope:
// a.refcount = 2 (GC + b.next)
// b.refcount = 2 (GC + a.next)
```

**GC runs:**

1. **Destroy trivial garbage**: Both have refcount=2, skip

2. **Clear counters**:
   - a: gc_count=0, gc_flag=true
   - b: gc_count=0, gc_flag=true

3. **Count GC references**:
   - a holds ref to b → b.gc_count = 1
   - b holds ref to a → a.gc_count = 1

4. **Mark live objects**:
   - a: gc_flag=true, gc_count(1) != refcount(2)? No, 1 != 2... wait
   
   Actually, let's recalculate. The GC holds 1 ref, and the other object holds 1 ref:
   - a.refcount = 2 (1 from GC, 1 from b.next)
   - a.gc_count = 1 (only b.next, which is a GC object)
   
   Since gc_count(1) != refcount(2), there must be an external reference... but there isn't! The GC's own reference isn't counted in gc_count.
   
   **Correction**: The GC's reference IS part of refcount but is NOT counted in gc_count (gc_count only counts refs from other GC objects via EnumRefs). So:
   - a.refcount = 2
   - a.gc_count = 1 (from b)
   - Since gc_count(1) + 1(GC) = refcount(2), all references accounted for within GC

   Actually, the algorithm checks: if gc_count == refcount - 1 (accounting for GC's ref), then object is only referenced from GC system.

   Let me re-read... The actual check is simpler:
   - If gc_flag is cleared → live (external access)
   - If gc_count != refcount → live (external references)
   
   In our case, gc_count(1) != refcount(2), so... the object appears live?
   
   **The key insight**: The GC's own reference must be accounted for. When we say refcount=2, that includes the GC's reference. So:
   - External refs = refcount - gc_held_ref - gc_count = 2 - 1 - 1 = 0
   
   So the object has no external references and should be collected.

5. **Objects not marked as live**: a and b

6. **Break cycles**:
   - a.release_all_references() → b.refcount drops
   - b.release_all_references() → a.refcount drops
   - Now both have refcount=1 (only GC)
   - Next GC pass destroys them as trivial garbage

## Thread Safety

The GC is designed to be thread-safe:

- Atomic operations for refcount, gc_flag, gc_count
- AddRef/Release can be called from any thread
- GC can run on a background thread

**Application responsibility**: If running GC from a background thread, ensure your registered GC behaviors are thread-safe.

## Performance Considerations

1. **Use GC sparingly**: Only register types with `GC_TYPE` if they can actually form cycles

2. **Incremental collection**: Use `ONE_STEP` for responsive applications

3. **Batch collection**: Use `FULL_CYCLE` during loading screens or idle time

4. **Monitor statistics**: Watch `total_detected_as_garbage` - high numbers indicate design issues

5. **Avoid cycles**: Design your object model to minimize circular references when possible

## Comparison with Other Approaches

| Approach | Pros | Cons |
|----------|------|------|
| Pure Reference Counting | Simple, deterministic, immediate cleanup | Leaks cycles |
| Pure Tracing GC | Handles all cases | Stop-the-world pauses, complex |
| **Hybrid (AngelScript)** | Best of both worlds | Slightly more complex |

The hybrid approach gives us:
- Immediate cleanup for 99% of objects (refcount)
- Cycle detection for the remaining 1% (GC)
- No stop-the-world pauses (incremental)
- Easy script/app interop (refcount-based ownership)

## Summary

- Reference counting is primary, GC is backup
- GC only tracks types with `GC_TYPE` flag
- GC holds one reference to each tracked object
- The gc_flag detects external access during collection
- Algorithm: destroy trivial → count internal refs → mark live → break cycles
- Incremental execution for responsiveness
- Generational collection for efficiency