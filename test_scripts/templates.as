// Test template type usage

class Object {}

void testBasicTemplates() {
    // Single template parameter
    array<int> intArray;
    array<string> stringArray;
    array<float> floatArray;
    
    intArray.insertLast(42);
    stringArray.insertLast("hello");
    floatArray.insertLast(3.14);
    
    // Dictionary template
    dictionary<string, int> ages;
    ages.set("Alice", 30);
    ages.set("Bob", 25);
    
    int age;
    if (ages.get("Alice", age)) {
        print("Alice is " + age);
    }
}

void testNestedTemplates() {
    // Array of arrays
    array<array<int>> matrix;
    matrix.resize(3);
    for (uint i = 0; i < 3; i++) {
        matrix[i].resize(3);
    }
    
    // Array of dictionaries
    array<dictionary<string, float>> records;
    
    // Dictionary of arrays
    dictionary<string, array<int>> groups;
    groups["primes"] = array<int>();
    groups["primes"].insertLast(2);
    groups["primes"].insertLast(3);
    groups["primes"].insertLast(5);
}

void testTripleNesting() {
    // Three levels of nesting - tests >> token splitting
    array<array<array<int>>> cube;
    cube.resize(3);
    for (uint i = 0; i < 3; i++) {
        cube[i].resize(3);
        for (uint j = 0; j < 3; j++) {
            cube[i][j].resize(3);
        }
    }
    
    // Access nested element
    cube[0][1][2] = 42;
    int value = cube[0][1][2];
}

void testTemplateHandles() {
    // Array of handles
    array<Object@> objects;
    
    // Array of const handles
    array<const Object@> constObjects;
    
    // Array of handles to const
    array<Object@ const> handlesToConst;
    
    // Complex: array of const handles to const objects
    array<const Object@ const> complexHandles;
}


void testComplexTemplateTypes() {
    // Dictionary with template value type
    dictionary<string, array<int>> groups;
    
    // Array of dictionaries with complex types
    array<dictionary<string, array<float>>> data;
    
    // Mixed template parameters
    dictionary<int, array<string>> mapping;
}

void testRightShiftAmbiguity() {
    // These should be parsed as template closing, not right shift
    array<array<int>> nested1;
    array<array<array<int>>> nested2;
    array<dictionary<string, array<int>>> nested3;
    
    // This should be right shift
    int shifted = 16 >> 2;
    
    // This context requires careful parsing
    array<array<int>> arr;
    int x = 8 >> 1;  // Right shift
}
