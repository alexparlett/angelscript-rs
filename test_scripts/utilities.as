// Utility functions - demonstrates common helper functions

// FFI placeholder - will be replaced with proper FFI bindings
float sqrt(float x) { return x; }
int rand() { return 0; }
const int RAND_MAX = 32767;

namespace Math {
    float clamp(float value, float min, float max) {
        if (value < min) return min;
        if (value > max) return max;
        return value;
    }
    
    int clampInt(int value, int min, int max) {
        if (value < min) return min;
        if (value > max) return max;
        return value;
    }
    
    float lerp(float a, float b, float t) {
        return a + (b - a) * clamp(t, 0.0, 1.0);
    }
    
    float smoothstep(float edge0, float edge1, float x) {
        float t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
        return t * t * (3.0 - 2.0 * t);
    }
    
    float distance(float x1, float y1, float x2, float y2) {
        float dx = x2 - x1;
        float dy = y2 - y1;
        return sqrt(dx * dx + dy * dy);
    }
}

namespace String {
    bool isEmpty(const string &in str) {
        return str.length() == 0;
    }
    
    bool startsWith(const string &in str, const string &in prefix) {
        if (prefix.length() > str.length()) return false;
        return str.substr(0, prefix.length()) == prefix;
    }
    
    bool endsWith(const string &in str, const string &in suffix) {
        if (suffix.length() > str.length()) return false;
        int start = str.length() - suffix.length();
        return str.substr(start) == suffix;
    }
    
    string trim(const string &in str) {
        // Remove leading and trailing whitespace
        int start = 0;
        int end = str.length() - 1;
        
        while (start < str.length() && isWhitespace(str[start])) {
            start++;
        }
        
        while (end >= 0 && isWhitespace(str[end])) {
            end--;
        }
        
        if (start > end) return "";
        return str.substr(start, end - start + 1);
    }
    
    bool isWhitespace(int c) {
        return c == 32 || c == 9 || c == 10 || c == 13;
    }
    
    array<string> split(const string &in str, const string &in delimiter) {
        array<string> result;
        int start = 0;
        int pos = str.findFirst(delimiter, start);
        
        while (pos >= 0) {
            result.insertLast(str.substr(start, pos - start));
            start = pos + delimiter.length();
            pos = str.findFirst(delimiter, start);
        }
        
        if (start < str.length()) {
            result.insertLast(str.substr(start));
        }
        
        return result;
    }
    
    string join(const array<string> &in parts, const string &in separator) {
        string result = "";
        for (uint i = 0; i < parts.length(); i++) {
            if (i > 0) result += separator;
            result += parts[i];
        }
        return result;
    }
}

namespace Array {
    void reverse(array<int> &inout arr) {
        int n = arr.length();
        for (int i = 0; i < n / 2; i++) {
            int temp = arr[i];
            arr[i] = arr[n - 1 - i];
            arr[n - 1 - i] = temp;
        }
    }
    
    bool contains(const array<int> &in arr, const T &in value) {
        for (uint i = 0; i < arr.length(); i++) {
            if (arr[i] == value) return true;
        }
        return false;
    }
    
    int indexOf(const array<int> &in arr, const T &in value) {
        for (uint i = 0; i < arr.length(); i++) {
            if (arr[i] == value) return int(i);
        }
        return -1;
    }

    funcdef bool Predicate(int value);
    
    array<int> filter(const array<int> &in arr, Predicate pred) {
        array<T> result;
        for (uint i = 0; i < arr.length(); i++) {
            if (Predicate(arr[i])) {
                result.insertLast(arr[i]);
            }
        }
        return result;
    }
}

namespace Random {
    int range(int min, int max) {
        return min + (rand() % (max - min + 1));
    }
    
    float rangeFloat(float min, float max) {
        float r = float(rand()) / float(RAND_MAX);
        return min + r * (max - min);
    }
    
    bool chance(float probability) {
        return rangeFloat(0.0, 1.0) < probability;
    }
    
    int choice(const array<int> &in options) {
        int index = range(0, options.length() - 1);
        return options[index];
    }
}

void testUtilities() {
    // Math utilities
    float clamped = Math::clamp(150.0, 0.0, 100.0);
    float lerped = Math::lerp(0.0, 100.0, 0.5);
    float dist = Math::distance(0, 0, 3, 4);
    
    // String utilities
    string str = "  hello world  ";
    string trimmed = String::trim(str);
    bool starts = String::startsWith("hello", "he");
    array<string> parts = String::split("a,b,c", ",");
    
    // Array utilities
    array<int> numbers = {1, 2, 3, 4, 5};
    Array::reverse(numbers);
    bool has3 = Array::contains(numbers, 3);
    int idx = Array::indexOf(numbers, 3);
    
    // Random utilities
    int randomNum = Random::range(1, 10);
    float randomFloat = Random::rangeFloat(0.0, 1.0);
    bool happened = Random::chance(0.5);
}
