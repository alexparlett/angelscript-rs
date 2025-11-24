// Test all literal types in AngelScript

void testLiterals() {
    // Integer literals
    int dec = 42;
    int hex = 0x2A;
    int oct = 052;
    int bin = 0b101010;
    
    // Floating point literals
    float f1 = 3.14;
    float f2 = 3.14f;
    double d1 = 2.71828;
    double d2 = 1.5e10;
    double d3 = 3.5e-5;
    
    // Boolean literals
    bool t = true;
    bool f = false;
    
    // String literals
    string s1 = "Hello";
    string s2 = 'World';
    string s3 = """Multi-line
string literal""";
    
    // Null literal
    Object@ obj = null;
}
