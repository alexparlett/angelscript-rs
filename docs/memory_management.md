# AngelScript Memory Management in Rust

## Overview

This document explains the memory management system for the AngelScript implementation in Rust. The design uses a **HashMap-based approach** for object properties, avoiding pointer arithmetic and byte offsets entirely.

---

## Core Principles

### 1. **Separation of Concerns**

The memory system is divided into three distinct storage areas:

| Storage Area | Purpose | Access Method | Performance |
|--------------|---------|---------------|-------------|
| **Stack Frames** | Local variables, temporaries, function parameters | Indexed by `u32` slot number | Fast (direct array access) |
| **Value Stack** | Argument passing, temporary expression values | Push/pop operations | Fast (Vec operations) |
| **Object Heap** | All object instances (script and Rust types) | HashMap lookup by object handle | Moderate (hash lookup) |

### 2. **Everything is a Handle**

All objects (both script-defined and Rust-backed) are **heap-allocated** and referenced by handles:

```rust
pub enum ScriptValue {
    // Primitives (stored by value)
    Int32(i32),
    Float(f32),
    String(String),
    // ... other primitives
    
    // Objects (always stored as handles)
    ObjectHandle(u64),  // Points to heap-allocated object
    
    Null,
}
```

**Why handles?**
- Enables references: `Foo@ ref = obj;`
- Supports shared ownership
- Simplifies memory management
- Allows reference counting

---

## Memory Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      VM MEMORY                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  CALL STACK (Function Frames)                          │
│  ┌───────────────────────────────────────────────┐    │
│  │ Frame N:                                       │    │
│  │   locals: Vec<ScriptValue>                    │    │
│  │     [0] Int32(42)         <- parameter        │    │
│  │     [1] ObjectHandle(100) <- local object     │    │
│  │     [2] Float(3.14)       <- temporary        │    │
│  │   return_address: 1234                        │    │
│  ├───────────────────────────────────────────────┤    │
│  │ Frame N-1: ...                                │    │
│  └───────────────────────────────────────────────┘    │
│                                                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  VALUE STACK (Argument Passing)                        │
│  ┌───────────────────────────────────────────────┐    │
│  │ [top]    Int32(10)        <- arg 2            │    │
│  │ [top-1]  ObjectHandle(100) <- arg 1           │    │
│  └───────────────────────────────────────────────┘    │
│                                                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  OBJECT HEAP (HashMap-based Objects)                   │
│  ┌───────────────────────────────────────────────┐    │
│  │ [100] ScriptObject {                          │    │
│  │   type_id: Player,                            │    │
│  │   ref_count: 2,                               │    │
│  │   properties: HashMap {                       │    │
│  │     "health" -> Int32(100),                   │    │
│  │     "name" -> String("Hero"),                 │    │
│  │     "weapon" -> ObjectHandle(200),            │    │
│  │   }                                           │    │
│  │ }                                             │    │
│  │                                               │    │
│  │ [200] ScriptObject {                          │    │
│  │   type_id: Weapon,                            │    │
│  │   ref_count: 1,                               │    │
│  │   properties: HashMap {                       │    │
│  │     "damage" -> Int32(50),                    │    │
│  │   }                                           │    │
│  │ }                                             │    │
│  └───────────────────────────────────────────────┘    │
│                                                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  GLOBALS (Module-level Variables)                      │
│  ┌───────────────────────────────────────────────┐    │
│  │ [0] Int32(42)                                 │    │
│  │ [1] ObjectHandle(300)                         │    │
│  └───────────────────────────────────────────────┘    │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## Script Objects: Pure HashMap Storage

### Structure

```rust
pub struct ScriptObject {
    /// Type ID (references UnifiedTypeInfo)
    type_id: u32,
    
    /// ALL properties stored as HashMap
    /// NO pointers, NO offsets, NO byte arrays
    properties: HashMap<String, ScriptValue>,
    
    /// Reference count (for handle semantics)
    ref_count: Arc<RwLock<usize>>,
    
    /// For Rust types ONLY: optional backing Rust instance
    rust_backing: Option<Arc<RwLock<Box<dyn Any + Send + Sync>>>>,
    
    /// Unique object identifier
    object_id: u64,
}
```

### Key Features

1. **No Memory Layout Calculations**
    - No offsets, no alignment, no size calculations
    - Properties accessed by name via HashMap
    - VM doesn't care about memory layout

2. **Uninitialized by Default**
    - Objects start with empty HashMap
    - Bytecode constructor initializes fields
    - Matches AngelScript semantics

3. **Type-Agnostic Storage**
    - Same structure for all object types
    - Type information stored separately in TypeRegistry
    - Flexible and extensible

---

## Property Access

### Script Types (Direct HashMap Access)

```angelscript
class Player {
    int health = 100;
    string name = "Hero";
}

Player p;
p.health = 50;  // How does this work?
```

**Bytecode Generated:**
```
1. Alloc Player -> handle 100
2. StoreObj var=0              // p = handle 100
3. SetThisProperty "health", 100  // Initialize health
4. SetThisProperty "name", "Hero" // Initialize name
5. SetProperty var=0, "health", 50 // p.health = 50
```

**VM Execution:**
```rust
// SetProperty instruction
fn execute_set_property(&mut self, obj_var: u32, prop_name_id: u32, src_var: u32) {
    // Get object handle from local variable
    let obj_handle = self.current_frame().get_local(obj_var).as_object_handle()?;
    
    // Get value to set
    let value = self.current_frame().get_local(src_var).clone();
    
    // Get property name
    let prop_name = self.module.get_property_name(prop_name_id)?;
    
    // Get object from heap
    let object = self.heap.get_object_mut(obj_handle)?;
    
    // Set property (HashMap insert - NO OFFSETS!)
    object.set_property(prop_name, value);
}
```

### Rust Types (Accessor Functions)

```rust
// Rust side
struct Enemy {
    health: i32,
    position_x: f32,
}

// Register with accessors
engine.register_rust_type::<Enemy>("Enemy")
    .property("health",
        |e: &Enemy| e.health,           // Getter
        |e: &mut Enemy, v| e.health = v // Setter
    );
```

**VM Execution:**
```rust
fn execute_set_property(&mut self, obj_var: u32, prop_name_id: u32, src_var: u32) {
    let obj_handle = self.current_frame().get_local(obj_var).as_object_handle()?;
    let value = self.current_frame().get_local(src_var).clone();
    let prop_name = self.module.get_property_name(prop_name_id)?;
    
    let type_registry = self.type_registry.read().unwrap();
    let object = self.heap.get_object(obj_handle)?;
    let type_info = type_registry.get_type(object.type_id())?;
    
    if type_info.flags.contains(TypeFlags::RUST_TYPE) {
        // RUST TYPE: Use accessor
        if let Some(accessor) = type_info.rust_accessors.get(prop_name) {
            if let Some(setter) = &accessor.setter {
                drop(type_registry); // Release lock
                let object_mut = self.heap.get_object_mut(obj_handle)?;
                setter(object_mut, value); // Call Rust setter
            }
        }
    } else {
        // SCRIPT TYPE: Direct HashMap access
        drop(type_registry);
        let object_mut = self.heap.get_object_mut(obj_handle)?;
        object_mut.set_property(prop_name, value);
    }
}
```

---

## Reference Semantics

### Handles Enable Sharing

```angelscript
class Foo {
    int value = 42;
}

class Bar {
    Foo@ reference;  // Handle to Foo
}

void test() {
    Foo obj;           // Local object on heap
    Bar bar;
    @bar.reference = @obj;  // Share reference
}
```

**Memory Layout:**
```
Stack Frame:
  locals[0] = ObjectHandle(100)  // obj
  locals[1] = ObjectHandle(101)  // bar

Heap:
  [100] ScriptObject (Foo) {
    ref_count: 2,  // Referenced by 'obj' and 'bar.reference'
    properties: {
      "value" -> Int32(42)
    }
  }
  
  [101] ScriptObject (Bar) {
    ref_count: 1,
    properties: {
      "reference" -> ObjectHandle(100)  // Points to Foo
    }
  }
```

### Reference Counting

```rust
impl ScriptObject {
    pub fn add_ref(&self) {
        let mut count = self.ref_count.write().unwrap();
        *count += 1;
    }
    
    pub fn release(&self) -> bool {
        let mut count = self.ref_count.write().unwrap();
        *count = count.saturating_sub(1);
        *count == 0  // Return true if should be destroyed
    }
}
```

**Bytecode Instructions:**
```rust
RefCpy { dst, src }  // Copy handle and increment refcount
Free { var, func_id } // Release handle, call destructor if refcount == 0
```

---

## Local Variables vs Objects

### Primitives: Stack Storage

```angelscript
void test() {
    int x = 42;      // Stored directly in stack frame
    float y = 3.14;  // Stored directly in stack frame
}
```

**Stack Frame:**
```
locals: Vec<ScriptValue>
  [0] Int32(42)    // x
  [1] Float(3.14)  // y
```

### Objects: Heap Storage

```angelscript
void test() {
    Player p;  // Handle stored in stack, object on heap
}
```

**Stack Frame:**
```
locals: Vec<ScriptValue>
  [0] ObjectHandle(100)  // p (handle to heap object)
```

**Heap:**
```
[100] ScriptObject {
  type_id: Player,
  ref_count: 1,
  properties: { ... }
}
```

---

## Temporary Variables

### Allocation Strategy

```rust
impl Compiler {
    fn allocate_temp(&mut self, type_id: TypeId) -> u32 {
        // Try to reuse from pool
        if let Some(var) = self.temp_var_pool.pop() {
            return var;
        }
        
        // Allocate new slot
        let var = self.local_count;
        self.local_count += 1;
        
        let var_info = LocalVarInfo {
            index: var,
            type_id,
            is_temp: true,  // Mark as temporary
            // ...
        };
        
        self.local_vars.insert(format!("$temp{}", var), var_info);
        var
    }
    
    fn free_temp(&mut self, var: u32) {
        if self.is_temp_var(var) {
            self.temp_var_pool.push(var);  // Return to pool
        }
    }
}
```

### Example Usage

```angelscript
int result = (a + b) * (c + d);
```

**Bytecode:**
```
1. ADDi temp0, a, b        // temp0 = a + b
2. ADDi temp1, c, d        // temp1 = c + d
3. MULi result, temp0, temp1  // result = temp0 * temp1
4. [free temp0, temp1]     // Return to pool
```

---

## Nested Property Access

### Chaining Handles

```angelscript
class Position { float x, y; }
class Player { Position@ pos; }
class Game { Player@ player; }

Game game;
game.player.pos.x = 10.0;
```

**Bytecode:**
```
1. GetProperty game, "player", temp0     // temp0 = handle to Player
2. GetProperty temp0, "pos", temp1       // temp1 = handle to Position
3. SetProperty temp1, "x", 10.0          // pos.x = 10.0
```

**Memory:**
```
Heap:
  [100] Game {
    properties: {
      "player" -> ObjectHandle(200)
    }
  }
  
  [200] Player {
    properties: {
      "pos" -> ObjectHandle(300)
    }
  }
  
  [300] Position {
    properties: {
      "x" -> Float(10.0),
      "y" -> Float(0.0)
    }
  }
```

---

## Initialization

### Script Type Initialization

```angelscript
class Player {
    int health = 100;    // Inline initializer
    string name;         // No initializer
}

Player p;
```

**Bytecode:**
```
1. Alloc Player -> handle 100
2. StoreObj var=0                      // p = handle 100
3. SetThisProperty "health", 100       // Initialize health (from inline)
4. SetThisProperty "name", ""          // Initialize name (default)
```

**Object State:**
```rust
// After Alloc (UNINITIALIZED)
ScriptObject {
    type_id: Player,
    properties: {},  // Empty!
}

// After constructor
ScriptObject {
    type_id: Player,
    properties: {
        "health" -> Int32(100),
        "name" -> String("")
    }
}
```

### Rust Type Initialization

```rust
impl RustTypeFactory for EnemyFactory {
    fn create_instance(&self) -> HashMap<String, ScriptValue> {
        let mut props = HashMap::new();
        props.insert("health".to_string(), ScriptValue::Int32(100));
        props.insert("position_x".to_string(), ScriptValue::Float(0.0));
        props
    }
}
```

**Object Created:**
```rust
ScriptObject {
    type_id: Enemy,
    properties: {
        "health" -> Int32(100),
        "position_x" -> Float(0.0)
    },
    rust_backing: Some(Box<Enemy>)  // Actual Rust instance
}
```

---

## Garbage Collection

### Reference Counting

Objects are destroyed when their reference count reaches zero:

```rust
impl ObjectHeap {
    pub fn release_object(&mut self, object_id: u64) -> bool {
        if let Some(object) = self.objects.get(&object_id) {
            if object.release() {  // Decrement refcount
                self.objects.remove(&object_id);  // Destroy if zero
                return true;
            }
        }
        false
    }
}
```

### Mark and Sweep (Optional)

For cyclic references:

```rust
impl ObjectHeap {
    pub fn collect_garbage(&mut self, root_handles: &[u64]) {
        // Mark phase: find all reachable objects
        let mut reachable = HashSet::new();
        let mut to_visit = root_handles.to_vec();
        
        while let Some(handle) = to_visit.pop() {
            if reachable.insert(handle) {
                if let Some(object) = self.objects.get(&handle) {
                    // Find all object handles in properties
                    for value in object.properties().values() {
                        if let ScriptValue::ObjectHandle(child) = value {
                            to_visit.push(*child);
                        }
                    }
                }
            }
        }
        
        // Sweep phase: remove unreachable objects
        self.objects.retain(|id, _| reachable.contains(id));
    }
}
```

---

## Performance Considerations

### Fast Operations

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Local variable access | O(1) | Direct array index |
| Stack push/pop | O(1) | Vec operations |
| Primitive operations | O(1) | Direct computation |

### Moderate Operations

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Property access | O(1) avg | HashMap lookup |
| Object allocation | O(1) | HashMap insert |
| Reference counting | O(1) | Atomic increment |

### Optimization Strategies

1. **Property Name Caching**
   ```rust
   // Compile time: register property name once
   let prop_id = module.add_property_name("health");
   
   // Runtime: reuse ID for fast lookup
   let prop_name = module.get_property_name(prop_id);
   ```

2. **Temporary Variable Pooling**
   ```rust
   // Reuse temporary slots instead of allocating new ones
   temp_var_pool: Vec<u32>
   ```

3. **Inline Property Storage**
   ```rust
   // Small objects could be stored inline (future optimization)
   enum ScriptValue {
       InlineObject { type_id: u32, data: [u8; 32] },
       ObjectHandle(u64),
   }
   ```

---

## Comparison with Traditional Approaches

### Traditional (C++ AngelScript)

```cpp
// Memory layout calculated at compile time
struct Player {
    int health;      // Offset 0
    float speed;     // Offset 4
    char* name;      // Offset 8
};

// Access via pointer arithmetic
void* obj = allocate(sizeof(Player));
*(int*)((char*)obj + 0) = 100;  // Set health
```

**Pros:**
- Fast (direct memory access)
- Cache-friendly (contiguous memory)

**Cons:**
- Complex (pointer arithmetic, alignment)
- Unsafe (buffer overflows, type confusion)
- Inflexible (layout fixed at compile time)

### Our Approach (Rust HashMap)

```rust
// No memory layout needed
let mut properties = HashMap::new();
properties.insert("health".to_string(), ScriptValue::Int32(100));
properties.insert("speed".to_string(), ScriptValue::Float(5.0));
```

**Pros:**
- Safe (no pointer arithmetic, bounds checked)
- Flexible (add/remove properties dynamically)
- Simple (no offset calculations)
- Rust-idiomatic (uses standard collections)

**Cons:**
- Slightly slower (hash lookup vs direct access)
- More memory (HashMap overhead)

**Trade-off:** We prioritize **safety and simplicity** over raw performance. For most use cases, the performance difference is negligible.

---

## Summary

### Key Takeaways

1. **No Pointers or Offsets**
    - All property access via HashMap
    - No memory layout calculations
    - Type-safe and bounds-checked

2. **Everything is a Handle**
    - Objects always heap-allocated
    - Handles enable sharing and references
    - Reference counting manages lifetime

3. **Hybrid Storage**
    - Primitives: direct storage in Vec
    - Objects: handles to heap
    - Best of both worlds

4. **Unified System**
    - Same approach for script and Rust types
    - Rust types use accessors
    - Script types use direct HashMap

5. **Safety First**
    - No unsafe code in memory management
    - Rust's type system prevents errors
    - Clear ownership semantics

This design provides a **safe, flexible, and maintainable** memory management system that stays true to AngelScript's semantics while leveraging Rust's strengths.