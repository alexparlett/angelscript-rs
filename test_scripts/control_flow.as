// Test all control flow constructs

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
        print("{}", i);
        i++;
    }

    int j = 0;
    do {
        print("{}", j);
        j++;
    } while (j < 5);
}

void testFor() {
    for (int i = 0; i < 10; i++) {
        print("{}", i);
    }

    // Infinite loop (commented out)
    // for (;;) { }

    // With complex expressions
    for (int i = 0, j = 10; i < j; i++, j--) {
        print("{}", i + j);
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

void testSwitchBool() {
    bool flag = true;

    switch (flag) {
        case true:
            print("flag is true");
            break;
        case false:
            print("flag is false");
            break;
    }
}

void testSwitchFloat() {
    float f = 1.5f;

    switch (f) {
        case 1.0f:
            print("one point zero");
            break;
        case 1.5f:
            print("one point five");
            break;
        case 2.0f:
            print("two point zero");
            break;
        default:
            print("other float");
            break;
    }
}

void testSwitchString() {
    string s = "hello";

    switch (s) {
        case "hello":
            print("greeting");
            break;
        case "goodbye":
            print("farewell");
            break;
        case "":
            print("empty string");
            break;
        default:
            print("unknown string");
            break;
    }
}

// Classes for type pattern matching tests
class Animal {
    Animal() {}
    void speak() { print("..."); }
}

class Dog : Animal {
    Dog() { super(); }
    void speak() { print("woof"); }
}

class Cat : Animal {
    Cat() { super(); }
    void speak() { print("meow"); }
}

void testSwitchHandleNull() {
    Animal@ pet = null;

    switch (pet) {
        case null:
            print("no pet");
            break;
        default:
            print("has a pet");
            break;
    }
}

void testSwitchTypePattern() {
    Dog@ dog = Dog();
    Animal@ pet = dog;

    switch (pet) {
        case Dog:
            print("it's a dog");
            break;
        case Cat:
            print("it's a cat");
            break;
        case null:
            print("no animal");
            break;
        default:
            print("some other animal");
            break;
    }
}

void testSwitchTypePatternCat() {
    Cat@ cat = Cat();
    Animal@ pet = cat;

    switch (pet) {
        case Dog:
            print("it's a dog");
            break;
        case Cat:
            print("it's a cat");
            break;
        default:
            print("unknown animal");
            break;
    }
}

void testBreakContinue() {
    for (int i = 0; i < 10; i++) {
        if (i == 3) continue;
        if (i == 7) break;
        print("{}", i);
    }
}

void testReturn() {
    int x = 5;
    if (x < 0) return;
    print("{}", x);
}
