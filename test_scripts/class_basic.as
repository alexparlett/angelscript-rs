// Test basic class declaration

class Point {
    private int x;
    private int y;
    
    Point() {
        x = 0;
        y = 0;
    }
    
    Point(int _x, int _y) {
        x = _x;
        y = _y;
    }
    
    ~Point() {
        // Destructor
    }
    
    void setX(int value) {
        x = value;
    }
    
    int getX() const {
        return x;
    }
    
    void setY(int value) {
        y = value;
    }
    
    int getY() const {
        return y;
    }
    
    float distance() const {
        return sqrt(x * x + y * y);
    }
    
    void move(int dx, int dy) {
        x += dx;
        y += dy;
    }
}

class Circle {
    private Point center;
    private float radius;
    
    Circle(int x, int y, float r) {
        center = Point(x, y);
        radius = r;
    }
    
    float area() const {
        return 3.14159 * radius * radius;
    }
    
    float circumference() const {
        return 2 * 3.14159 * radius;
    }
}

void testClasses() {
    Point p1;
    Point p2(10, 20);
    
    p1.setX(5);
    p1.setY(7);
    
    int x = p2.getX();
    float dist = p1.distance();
    
    Circle c(0, 0, 5.0);
    float area = c.area();
}
