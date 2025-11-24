# Parser Performance Optimization Plan

Based on profiling data showing **Thread 15 (benchmark thread)** hot spots:

## Performance Bottlenecks Identified

**Total Target:** Need ~24% improvement to reach sub-1ms goal (currently 1.31ms → target <1ms)

### Hot Spots (by % of total time):
1. `Parser::fill_buffer` - **20.6%** (2556 samples)
2. `Lexer::scan_token` - **13.4%** (1665 samples)
3. `Cursor::eat_while` - **11.0%** (1363 samples)
4. `Cursor::advance` - **10.1%** (1247 samples)
5. `_platform_memmove` - **7.0%** (869 samples)
6. `Lexer::scan_operator` - **4.3%** (536 samples)

**Lexer total: 46.8%** of all time spent in lexing!

---

## Task Breakdown

### **Task 1: Optimize Parser::fill_buffer (Target: 10-15% improvement)**
**Impact:** 20.6% of total time
**Difficulty:** Medium
**Files:** `src/ast/parser.rs`

**Problem:**
- Calls `lexer.next_token()` in a loop for each token needed
- Small initial buffer capacity (32 tokens)
- Causes frequent allocations (memmove overhead)

**Solution:**
1. Pre-tokenize entire input upfront OR use much larger initial buffer
2. Change `buffer: Vec<Token<'src>>` capacity from 32 to 512 or 1024
3. Consider pre-filling buffer with all tokens at Parser::new()
4. Benchmark both approaches

**Changes:**
- Line 37: `buffer: Vec::with_capacity(512)` (or larger)
- Line 144-162: Optimize `fill_buffer()` logic - consider bulk tokenization

---

### **Task 2: Optimize Cursor::eat_while (Target: 5-8% improvement)**
**Impact:** 11.0% of total time
**Difficulty:** Medium-Hard
**Files:** `src/lexer/cursor.rs`

**Problem:**
- Lines 181-187: Uses iterator with `self.check(&f)` + `self.advance()` in loop
- Each iteration has UTF-8 boundary checking overhead
- Function call overhead for predicate

**Solution:**
1. Use unsafe pointer arithmetic for ASCII-only predicates
2. Batch character boundary validation
3. Specialize for common cases (alphanumeric, whitespace)

**Changes:**
- Add `eat_while_ascii` fast path for ASCII predicates
- Use `str::as_bytes()` + direct indexing where safe
- Only validate UTF-8 boundaries once at end

**Example approach:**
```rust
pub fn eat_while_ascii_fast(&mut self, predicate: impl Fn(u8) -> bool) -> &'src str {
    let start = self.offset as usize;
    let bytes = self.rest.as_bytes();
    let mut i = 0;

    while i < bytes.len() && predicate(bytes[i]) {
        i += 1;
    }

    self.advance_bytes(i);
    &self.source[start..self.offset as usize]
}
```

---

### **Task 3: Optimize Cursor::advance (Target: 4-6% improvement)**
**Impact:** 10.1% of total time
**Difficulty:** Medium
**Files:** `src/lexer/cursor.rs`

**Problem:**
- Lines 119-134: UTF-8 char boundary checking on every call
- `char.len_utf8()` computed twice (once for next(), once explicitly)
- Branching for newline tracking

**Solution:**
1. Cache the char length from `chars().next()`
2. Use branchless column increment where possible
3. Inline aggressively

**Changes:**
- Line 120-121: Combine char extraction and length calculation
- Add `#[inline(always)]` attribute
- Consider separate fast-path for ASCII-only advancement

---

### **Task 4: Optimize Lexer::scan_operator (Target: 2-3% improvement)**
**Impact:** 4.3% of total time
**Difficulty:** Easy-Medium
**Files:** `src/lexer/lexer.rs`

**Problem:**
- Likely has many branches for different operators
- Could use lookup table or perfect hash

**Solution:**
1. Profile to see which operators are most common
2. Reorder branches to put common cases first
3. Consider jump table for single-char operators

**Changes:**
- Reorder match arms by frequency
- Use lookup table for single-byte operators

---

### **Task 5: Reduce Vec Growth / memmove Overhead (Target: 3-5% improvement)**
**Impact:** 7.0% of total time
**Difficulty:** Easy
**Files:** `src/ast/parser.rs`, `src/lexer/lexer.rs`

**Problem:**
- Parser buffer starts at capacity 32 (line 37)
- Lexer lookahead starts at capacity 4 (lexer.rs line 33)
- 5000-line file likely needs 10,000+ tokens
- Frequent reallocation triggers memmove

**Solution:**
1. Increase initial capacities significantly
2. Pre-allocate based on source length heuristic

**Changes:**
- Parser buffer: `Vec::with_capacity(1024)` or `Vec::with_capacity(source.len() / 5)`
- Lexer lookahead: Keep small (only for peeking)

---

## Execution Order

**Phase 1 (Quick Wins):**
1. Task 5 (Reduce Vec growth) - easiest, immediate impact
2. Task 1 (fill_buffer optimization) - straightforward capacity changes

**Phase 2 (Medium Effort):**
3. Task 4 (scan_operator) - branch reordering
4. Task 3 (Cursor::advance) - inlining and micro-optimizations

**Phase 3 (If Needed):**
5. Task 2 (Cursor::eat_while) - more complex unsafe optimizations

**Expected Total Improvement:** 24-37% faster → should easily hit sub-1ms target

---

## Validation

After each task:
1. Run `cargo bench --bench parser_benchmarks`
2. Verify stress_5000_lines benchmark time
3. Ensure all 489 tests still pass
4. Profile again if target not reached
