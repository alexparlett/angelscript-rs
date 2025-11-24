// Test operator precedence and associativity

void testOperators() {
    // Arithmetic operators
    int a = 1 + 2 * 3;        // Should be 7 (multiplication first)
    int b = (1 + 2) * 3;      // Should be 9 (explicit grouping)
    int c = 10 - 5 + 2;       // Should be 7 (left-to-right)
    int d = 20 / 4 * 2;       // Should be 10 (left-to-right)
    int e = 2 ** 3 ** 2;      // Power is right-associative: 2 ** (3 ** 2) = 512
    
    // Comparison operators
    bool cmp1 = 5 > 3;
    bool cmp2 = 10 <= 10;
    bool cmp3 = 7 == 7;
    bool cmp4 = 8 != 9;
    
    // Logical operators
    bool log1 = true && false || true;   // Should be true
    bool log2 = true || false && false;  // Should be true (AND has higher precedence)
    bool log3 = !false;                   // Should be true
    
    // Bitwise operators
    int bit1 = 0xFF & 0x0F;
    int bit2 = 0xF0 | 0x0F;
    int bit3 = 0xFF ^ 0x0F;
    int bit4 = ~0xFF;
    int bit5 = 1 << 3;
    int bit6 = 16 >> 2;
    int bit7 = -16 >>> 2;  // Unsigned shift
    
    // Assignment operators
    int x = 10;
    x += 5;   // x = 15
    x -= 3;   // x = 12
    x *= 2;   // x = 24
    x /= 4;   // x = 6
    x %= 4;   // x = 2
    x &= 3;   // x = 2
    x |= 1;   // x = 3
    x ^= 2;   // x = 1
    x <<= 2;  // x = 4
    x >>= 1;  // x = 2
    x >>>= 1; // x = 1
    
    // Increment/decrement
    int y = 5;
    ++y;  // pre-increment
    y++;  // post-increment
    --y;  // pre-decrement
    y--;  // post-decrement
    
    // Ternary operator
    int z = (y > 0) ? y : -y;
    
    // Member and indexing
    // Object obj;
    // int val = obj.member;
    // int elem = array[index];
}
