// Test all control flow constructs

// FFI placeholder - will be replaced with proper FFI bindings
void print(const string &in msg) {}
void print(int value) {}

void testIf() {
    int x = 10;
    
    if (x > 5) {
        print("x is greater than 5");
    }
    
    if (x < 5) {
        print("x is less than 5");
    } else {
        print("x is not less than 5");
    }
    
    if (x == 0) {
        print("zero");
    } else if (x > 0) {
        print("positive");
    } else {
        print("negative");
    }
}

void testWhile() {
    int i = 0;
    while (i < 10) {
        print(i);
        i++;
    }
    
    int j = 0;
    do {
        print(j);
        j++;
    } while (j < 5);
}

void testFor() {
    for (int i = 0; i < 10; i++) {
        print(i);
    }
    
    // Infinite loop (commented out)
    // for (;;) { }
    
    // With complex expressions
    for (int i = 0, j = 10; i < j; i++, j--) {
        print(i + j);
    }
}

void testSwitch() {
    int value = 2;
    
    switch (value) {
        case 0:
            print("zero");
            break;
        case 1:
            print("one");
            break;
        case 2:
            print("two");
            break;
        default:
            print("other");
            break;
    }
}

void testBreakContinue() {
    for (int i = 0; i < 10; i++) {
        if (i == 3) continue;
        if (i == 7) break;
        print(i);
    }
}

void testReturn() {
    int x = 5;
    if (x < 0) return;
    print(x);
}
