// Test various type expressions

// Primitive types
int primitiveInt;
uint primitiveUInt;
float primitiveFloat;
double primitiveDouble;
bool primitiveBool;
string primitiveString;

// Const types
const int constInt = 42;
const string constString = "immutable";

// Array types
int[] simpleArray;
array<int> arrayTemplate;
array<array<int>> nestedArray;

// Handle types
Object@ simpleHandle;
Object@ const constHandle;
const Object@ handleToConst;
const Object@ const constHandleToConst;

// Reference types
void funcWithRef(int &ref) {}
void funcWithInRef(int &in inRef) {}
void funcWithOutRef(int &out outRef) {}
void funcWithInOutRef(int &inout inoutRef) {}

// Scoped types
namespace N {
    class Inner {}
}
N::Inner scopedType;

// Template types
array<int> intArray;
dictionary<string, int> stringToInt;

// Complex nested types
array<const Object@ const>@ arrayOfHandles;
const array<array<int>>@ const complexType;

// Function pointers / Funcdefs
funcdef void Callback();
funcdef int BinaryOp(int, int);

Callback@ callbackPtr;
BinaryOp@ binaryOpPtr;

// Typedefs
typedef int EntityId;
typedef array<string> StringArray;

EntityId id;
StringArray names;

// Auto type (type inference)
auto inferredInt = 42;
auto inferredString = "hello";
