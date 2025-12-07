// Test using namespace directive
// Tests importing namespace symbols into current scope

// Define some namespaces with types and functions
namespace test {
    void func() {}

    class Entity {
        int id;

        Entity(int _id) {
            id = _id;
        }

        int getId() const { return id; }
    }

    enum Color {
        Red,
        Green,
        Blue
    }
}

namespace utils {
    void helper() {}

    class Logger {
        void log(const string &in msg) {
            print(msg);
        }
    }
}

namespace nested::inner {
    void deepFunc() {}

    class DeepClass {
        int value;

        DeepClass(int v) { value = v; }
    }
}

// Import test namespace at global scope
using namespace test;

// Test basic usage - types and functions from imported namespace
void testBasicUsing() {
    func();  // Should find test::func

    Entity e(42);  // Should find test::Entity
    int id = e.getId();

    Color c = Red;  // Should find test::Color and test::Red
}

// Test that explicit qualification still works
void testExplicitQualification() {
    test::func();
    test::Entity e(1);
    test::Color c = test::Color::Green;  // Fully qualified enum value

    // Also test other namespaces
    utils::helper();
    utils::Logger logger;
}

// Test nested namespace import
using namespace nested::inner;

void testNestedImport() {
    deepFunc();  // Should find nested::inner::deepFunc
    DeepClass dc(100);  // Should find nested::inner::DeepClass
}

// Test scoped import inside a namespace
namespace myspace {
    using namespace utils;

    void testScopedImport() {
        helper();  // Should find utils::helper via import
        Logger logger;  // Should find utils::Logger via import
    }
}

// Main test function
void main() {
    testBasicUsing();
    testExplicitQualification();
    testNestedImport();
    myspace::testScopedImport();
    print("Using namespace tests passed!");
}
