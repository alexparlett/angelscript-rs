// Test interface declarations

interface IComparable {
    int compareTo(IComparable@ other);
}

interface ICloneable {
    ICloneable@ clone();
}

interface ISerializable {
    string serialize();
    void deserialize(string data);
}

interface IDrawable {
    void draw();
    void setVisible(bool visible);
    bool isVisible() const;
}

interface IUpdatable {
    void update(float deltaTime);
}

// Interface with properties (if supported)
interface IEntity {
    int getId() const;
    string getName() const;
    void setName(string name);
}

// Class implementing multiple interfaces
class GameObject : IDrawable, IUpdatable {
    private bool visible;
    private float x;
    private float y;
    
    GameObject() {
        visible = true;
        x = 0;
        y = 0;
    }
    
    void draw() {
        if (visible) {
            print("Drawing at " + x + ", " + y);
        }
    }
    
    void setVisible(bool v) {
        visible = v;
    }
    
    bool isVisible() const {
        return visible;
    }
    
    void update(float deltaTime) {
        x += deltaTime * 10;
    }
}

class Player : IEntity, IDrawable {
    private int id;
    private string name;
    private bool visible;
    
    Player(int _id, string _name) {
        id = _id;
        name = _name;
        visible = true;
    }
    
    int getId() const {
        return id;
    }
    
    string getName() const {
        return name;
    }
    
    void setName(string n) {
        name = n;
    }
    
    void draw() {
        if (visible) {
            print("Drawing player: " + name);
        }
    }
    
    void setVisible(bool v) {
        visible = v;
    }
    
    bool isVisible() const {
        return visible;
    }
}

void testInterfaces() {
    GameObject obj;
    IDrawable@ drawable = @obj;
    drawable.draw();
    
    IUpdatable@ updatable = @obj;
    updatable.update(0.016);
    
    Player player(1, "Hero");
    IEntity@ entity = @player;
    print(entity.getName());
}
