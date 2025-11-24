// Test class inheritance

class Animal {
    protected string name;
    protected int age;
    
    Animal(string n, int a) {
        name = n;
        age = a;
    }
    
    void speak() {
        print("Animal makes a sound");
    }
    
    string getName() const {
        return name;
    }
}

class Dog : Animal {
    private string breed;
    
    Dog(string n, int a, string b) {
        super(n, a);
        breed = b;
    }
    
    void speak() {
        print("Woof!");
    }
    
    void wagTail() {
        print("Wagging tail");
    }
}

class Cat : Animal {
    private bool isIndoor;
    
    Cat(string n, int a, bool indoor) {
        super(n, a);
        isIndoor = indoor;
    }
    
    void speak() {
        print("Meow!");
    }
    
    void purr() {
        print("Purring");
    }
}

// Multiple inheritance
interface IDrawable {
    void draw();
}

interface IUpdatable {
    void update(float dt);
}

class GameObject : IDrawable, IUpdatable {
    private float x;
    private float y;
    
    void draw() {
        print("Drawing at " + x + ", " + y);
    }
    
    void update(float dt) {
        x += dt;
    }
}

// Diamond problem (if class-based multiple inheritance supported)
class Flyable {
    void fly() {
        print("Flying");
    }
}

class Swimmable {
    void swim() {
        print("Swimming");
    }
}

class Duck : Flyable, Swimmable {
    void quack() {
        print("Quack!");
    }
}

void testInheritance() {
    Dog dog("Buddy", 5, "Golden Retriever");
    dog.speak();
    dog.wagTail();
    
    Cat cat("Whiskers", 3, true);
    cat.speak();
    cat.purr();
    
    GameObject obj;
    obj.draw();
    obj.update(1.0);
    
    Duck duck;
    duck.fly();
    duck.swim();
    duck.quack();
}
