// Lambda Expressions Test Script
// Tests various lambda expression syntax and usage patterns

// Basic funcdef declarations
funcdef int BinaryOp(int a, int b);
funcdef void Callback(int);
funcdef int Transformer(int);
funcdef bool Predicate(int);

// Lambda with explicit parameter types
void test_explicit_types() {
    BinaryOp @add = function(int x, int y) { return x + y; };
    BinaryOp @multiply = function(int a, int b) { return a * b; };
}

// Lambda with inferred parameter types (name-only parameters)
void test_inferred_types() {
    BinaryOp @subtract = function(a, b) { return a - b; };
    BinaryOp @divide = function(x, y) { return x / y; };
}

// Lambda passed directly as function argument (inline lambda)
void applyOp(BinaryOp @op) {
    int result = op(10, 20);
}

void test_inline_lambda() {
    applyOp(function(a, b) { return a + b; });
    applyOp(function(a, b) { return a * b; });
}

// Multiple lambdas in same function
void test_multiple_lambdas() {
    Callback @cb1 = function(x) { };
    Callback @cb2 = function(y) { };
    Callback @cb3 = function(z) { };
}

// Lambda calling another lambda
void test_lambda_invocation() {
    BinaryOp @add = function(a, b) { return a + b; };
    int result = add(5, 3);
}

// Lambda with void return
void executeCallback(Callback @cb) {
    cb(42);
}

void test_void_lambda() {
    executeCallback(function(value) { });
}

// Lambda with complex body
void test_complex_lambda() {
    BinaryOp @max = function(a, b) {
        if (a > b) {
            return a;
        } else {
            return b;
        }
    };
}

// Lambda stored in local variable
void test_lambda_storage() {
    Predicate @isPositive = function(n) { return n > 0; };
    Predicate @isEven = function(n) { return n % 2 == 0; };

    bool result1 = isPositive(5);
    bool result2 = isEven(4);
}

// Nested lambda (lambda returning lambda)
funcdef Callback CallbackMaker();

void test_nested_lambda() {
    CallbackMaker @maker = function() {
        return function(x) { };
    };
}

// Lambda assigned to class member (if supported)
class Container {
    BinaryOp @operation;

    void setOperation(BinaryOp @op) {
        @operation = op;
    }
}

void test_lambda_in_class() {
    Container container;
    container.setOperation(function(a, b) { return a + b; });
}
