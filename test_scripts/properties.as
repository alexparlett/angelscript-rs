// Test virtual properties (get/set methods)

class Rectangle {
    private int width;
    private int height;
    
    Rectangle(int w, int h) {
        width = w;
        height = h;
    }
    
    // Read-write property
    int Width {
        get const { return width; }
        set { width = value; }
    }
    
    int Height {
        get const { return height; }
        set { height = value; }
    }
    
    // Read-only property
    int Area {
        get const { return width * height; }
    }
    
    // Computed property
    int Perimeter {
        get const { return 2 * (width + height); }
    }
}

class Player {
    private int health;
    private int maxHealth;
    private string name;
    
    Player(string n, int maxHp) {
        name = n;
        maxHealth = maxHp;
        health = maxHp;
    }
    
    // Read-only property
    string Name {
        get const { return name; }
    }
    
    // Read-write property with validation
    int Health {
        get const { return health; }
        set {
            if (value < 0) {
                health = 0;
            } else if (value > maxHealth) {
                health = maxHealth;
            } else {
                health = value;
            }
        }
    }
    
    // Read-only computed property
    float HealthPercent {
        get const {
            return float(health) / float(maxHealth) * 100.0;
        }
    }
    
    bool IsAlive {
        get const { return health > 0; }
    }
}

class Vector3 {
    private float x;
    private float y;
    private float z;
    
    Vector3(float _x, float _y, float _z) {
        x = _x;
        y = _y;
        z = _z;
    }
    
    float X {
        get const { return x; }
        set { x = value; }
    }
    
    float Y {
        get const { return y; }
        set { y = value; }
    }
    
    float Z {
        get const { return z; }
        set { z = value; }
    }
    
    // Computed length property
    float Length {
        get const {
            return sqrt(x*x + y*y + z*z);
        }
    }
    
    // Normalized property returns a new vector
    Vector3 Normalized {
        get const {
            float len = Length;
            if (len > 0) {
                return Vector3(x/len, y/len, z/len);
            }
            return Vector3(0, 0, 0);
        }
    }
}

void testProperties() {
    Rectangle rect(10, 20);
    
    // Using properties like fields
    rect.Width = 15;
    rect.Height = 25;
    
    int w = rect.Width;
    int h = rect.Height;
    int area = rect.Area;
    int perim = rect.Perimeter;
    
    Player player("Hero", 100);
    player.Health = 75;
    float healthPct = player.HealthPercent;
    bool alive = player.IsAlive;
    
    Vector3 v(3, 4, 0);
    float length = v.Length;
    Vector3 normalized = v.Normalized;
}
