// Game logic example - demonstrates real-world AngelScript usage

// FFI placeholder - will be replaced with proper FFI bindings
float sqrt(float x) { return x; }
float cos(float x) { return x; }
float sin(float x) { return x; }
float random(float min, float max) { return min; }

enum GameState {
    MainMenu,
    Playing,
    Paused,
    GameOver
}

class Player {
    private float x;
    private float y;
    private int health;
    private int maxHealth;
    private float speed;
    private int score;
    
    Player(float startX, float startY) {
        x = startX;
        y = startY;
        health = 100;
        maxHealth = 100;
        speed = 5.0;
        score = 0;
    }
    
    void move(float dx, float dy, float dt) {
        x += dx * speed * dt;
        y += dy * speed * dt;
    }
    
    void takeDamage(int amount) {
        health -= amount;
        if (health < 0) {
            health = 0;
        }
    }
    
    void heal(int amount) {
        health += amount;
        if (health > maxHealth) {
            health = maxHealth;
        }
    }
    
    bool isAlive() const {
        return health > 0;
    }
    
    void addScore(int points) {
        score += points;
    }
    
    int getScore() const {
        return score;
    }
    
    float getX() const { return x; }
    float getY() const { return y; }
}

class Enemy {
    private float x;
    private float y;
    private int health;
    private float speed;
    private int damage;
    
    Enemy(float startX, float startY, int hp, int dmg) {
        x = startX;
        y = startY;
        health = hp;
        speed = 3.0;
        damage = dmg;
    }
    
    void update(float dt, Player@ player) {
        // Move towards player
        float dx = player.getX() - x;
        float dy = player.getY() - y;
        float dist = sqrt(dx * dx + dy * dy);
        
        if (dist > 1.0) {
            x += (dx / dist) * speed * dt;
            y += (dy / dist) * speed * dt;
        }
    }
    
    bool checkCollision(Player@ player) {
        float dx = player.getX() - x;
        float dy = player.getY() - y;
        float dist = sqrt(dx * dx + dy * dy);
        return dist < 1.0;
    }
    
    void takeDamage(int amount) {
        health -= amount;
    }
    
    bool isAlive() const {
        return health > 0;
    }
    
    int getDamage() const {
        return damage;
    }
}

class Game {
    private GameState state;
    private Player@ player;
    private array<Enemy@> enemies;
    private float elapsedTime;
    
    Game() {
        state = GameState::MainMenu;
        elapsedTime = 0;
    }
    
    void startGame() {
        state = GameState::Playing;
        @player = Player(0, 0);
        
        // Spawn initial enemies
        spawnEnemy(10, 10, 50, 10);
        spawnEnemy(-10, 10, 50, 10);
        spawnEnemy(10, -10, 50, 10);
    }
    
    void spawnEnemy(float x, float y, int health, int damage) {
        enemies.insertLast(Enemy(x, y, health, damage));
    }
    
    void update(float dt) {
        if (state != GameState::Playing) {
            return;
        }
        
        elapsedTime += dt;
        
        // Update enemies
        for (uint i = 0; i < enemies.length(); i++) {
            if (enemies[i].isAlive()) {
                enemies[i].update(dt, player);
                
                // Check collision
                if (enemies[i].checkCollision(player)) {
                    player.takeDamage(enemies[i].getDamage());
                }
            }
        }
        
        // Remove dead enemies
        for (uint i = 0; i < enemies.length(); i++) {
            if (!enemies[i].isAlive()) {
                player.addScore(100);
                enemies.removeAt(i);
                i--;
            }
        }
        
        // Check game over
        if (!player.isAlive()) {
            state = GameState::GameOver;
        }
        
        // Spawn new enemies periodically
        if (int(elapsedTime) % 10 == 0) {
            float angle = random(0, 6.28);
            float dist = 20.0;
            spawnEnemy(cos(angle) * dist, sin(angle) * dist, 50, 10);
        }
    }
    
    void pause() {
        if (state == GameState::Playing) {
            state = GameState::Paused;
        }
    }
    
    void resume() {
        if (state == GameState::Paused) {
            state = GameState::Playing;
        }
    }
    
    GameState getState() const {
        return state;
    }
}

void main() {
    Game game;
    game.startGame();
    
    // Game loop would go here
    float dt = 0.016;
    game.update(dt);
}
