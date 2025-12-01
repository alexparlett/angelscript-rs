// Test function declarations with various parameters

// FFI placeholder - will be replaced with proper FFI bindings
void print(const string &in msg) {}
void print(int value) {}

void noParams() {
    print("No parameters");
}

int singleParam(int x) {
    return x * 2;
}

float multipleParams(int a, float b, string c) {
    print(c);
    return a + b;
}

void refParams(int &out result) {
    result = 42;
}

void inRefParams(const int &in value) {
    print(value);
}

void inoutParams(int &inout x) {
    x = x * 2;
}

int defaultParams(int x = 10, int y = 20) {
    return x + y;
}

int autoReturn() {
    return 42;
}

// Function overloading
void overloaded(int x) {
    print("int version");
}

void overloaded(float x) {
    print("float version");
}

void overloaded(int x, int y) {
    print("two int version");
}

// Const methods are in class context

// Function with complex return type
array<int> returnsArray() {
    array<int> arr;
    return arr;
}

// Template functions (if supported)
// void templateFunc<T>(T value) {}
