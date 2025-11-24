// Test nested classes and namespaces

namespace Game {
    class Entity {
        protected int id;
        protected string name;
        
        Entity(int _id, string _name) {
            id = _id;
            name = _name;
        }
        
        int getId() const { return id; }
        string getName() const { return name; }
    }
    
    namespace Physics {
        class Body {
            private float x;
            private float y;
            private float vx;
            private float vy;
            
            Body(float _x, float _y) {
                x = _x;
                y = _y;
                vx = 0;
                vy = 0;
            }
            
            void update(float dt) {
                x += vx * dt;
                y += vy * dt;
            }
        }
        
        class World {
            private array<Body@> bodies;
            
            void addBody(Body@ body) {
                bodies.insertLast(body);
            }
            
            void simulate(float dt) {
                for (uint i = 0; i < bodies.length(); i++) {
                    bodies[i].update(dt);
                }
            }
        }
    }
    
    namespace UI {
        class Widget {
            protected int x;
            protected int y;
            protected int width;
            protected int height;
            
            Widget(int _x, int _y, int _w, int _h) {
                x = _x;
                y = _y;
                width = _w;
                height = _h;
            }
            
            void draw() {
                // Draw widget
            }
        }
        
        class Button : Widget {
            private string text;
            
            Button(int x, int y, int w, int h, string t) {
                super(x, y, w, h);
                text = t;
            }
            
            void draw() {
                // Draw button
            }
            
            void onClick() {
                print("Button clicked: " + text);
            }
        }
    }
}

namespace Math {
    class Vector2 {
        float x;
        float y;
        
        Vector2(float _x, float _y) {
            x = _x;
            y = _y;
        }
        
        Vector2 opAdd(const Vector2 &in other) const {
            return Vector2(x + other.x, y + other.y);
        }
    }
    
    class Matrix {
        private array<array<float>> data;
        
        Matrix(int rows, int cols) {
            data.resize(rows);
            for (uint i = 0; i < rows; i++) {
                data[i].resize(cols);
            }
        }
    }
}

void testNested() {
    Game::Entity entity(1, "Player");
    
    Game::Physics::Body body(0, 0);
    body.update(0.016);
    
    Game::Physics::World world;
    world.addBody(@body);
    world.simulate(0.016);
    
    Game::UI::Button button(10, 10, 100, 30, "Click Me");
    button.draw();
    button.onClick();
    
    Math::Vector2 v1(1, 2);
    Math::Vector2 v2(3, 4);
    Math::Vector2 v3 = v1 + v2;
}
