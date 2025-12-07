// Test complex nested expressions

int max(int a, int b) { return a > b ? a : b; }
int min(int a, int b) { return a < b ? a : b; }
int abs(int x) { return x < 0 ? -x : x; }

void testComplexExpressions() {
    // Deeply nested arithmetic
    int result1 = ((1 + 2) * (3 - 4)) / ((5 + 6) * (7 - 8));
    
    // Mixed operators with precedence
    int result2 = 1 + 2 * 3 - 4 / 2 + 5 % 3;
    
    // Nested logical expressions
    bool result3 = (true && (false || true)) || (false && (true || false));
    
    // Bitwise combinations
    int result4 = ((0xFF & 0xF0) | (0x0F ^ 0xFF)) << 2;
    
    // Multiple ternary operators
    int x = 10;
    int result5 = x > 0 ? (x > 5 ? 1 : 2) : (x < -5 ? -1 : -2);
    
    // Chained comparisons (if supported)
    bool result6 = 1 < 5 && 5 < 10 && 10 < 100;
    
    // Complex assignments
    int a = 5;
    int b = (a += 3) * (a -= 1);
    
    // Function call expressions
    int result7 = max(min(x, 10), 0) + abs(x - 5);
    
    // Array and member access chains
    // array<array<int>> matrix;
    // int elem = matrix[i][j];
    
    // Complex object chains
    // player.inventory.items[slot].use();
    
    // Lambda expressions (if supported)
    // auto f = function(int x) { return x * 2; };
    // int result8 = f(5);
}

void testOperatorOverloading() {
    // Assuming Vector2 class with operator overloading
    // Vector2 v1(1, 2);
    // Vector2 v2(3, 4);
    // Vector2 v3 = v1 + v2 * 3 - v1;
    // float dot = v1.dot(v2);
    // Vector2 v4 = v1.cross(v2);
}

class ComplexCalculator {
    int compute(int a, int b, int c) {
        return ((a * b + c) / (a - b)) % (b + c);
    }
    
    float evaluate(float x) {
        return (x * x * x - 2 * x * x + 3 * x - 4) / (x + 1);
    }
    
    bool test(int value) {
        return (value > 0 && value < 100) ||
               (value % 2 == 0 && value % 3 == 0) ||
               ((value & 0xFF) == 0xFF);
    }
}

void testExpressionCombinations() {
    int x = 5, y = 10, z = 15;
    
    // Complex condition
    if ((x > 0 && y < 20) || (z >= 10 && x * y > 30)) {
        print("Complex condition true");
    }
    
    // Complex loop control
    for (int i = 0; i < 100 && (i * i < 1000 || i % 10 == 0); i++) {
        if (i % 2 == 0 && i % 3 == 0) continue;
        if (i > 50 && i < 60) break;
        print(i);
    }
    
    // Switch with complex expressions
    switch (x * y + z) {
        case 1 + 2:
            print("3");
            break;
        case 10 * 5:
            print("50");
            break;
        default:
            print("other");
            break;
    }
    
    // Nested casts (if supported)
    // float f = float(int(double(x) * 1.5));
}

void testShortCircuit() {
    int x = 0;
    
    // Should short-circuit and not evaluate second part
    if (x != 0 && 10 / x > 5) {
        print("Not executed");
    }
    
    // Should short-circuit
    if (x == 0 || 10 / x > 5) {
        print("Executed");
    }
}

void testPrecedenceEdgeCases() {
    int a = 1, b = 2, c = 3;
    
    // Unary operators
    int r1 = -a * +b + ~c;
    
    // Increment/decrement with other operators
    int r2 = ++a * b--;
    
    // Mixed assignment operators
    a += b *= c;
    
    // Comparison chaining (careful with precedence)
    bool r3 = a < b && b < c;
}
