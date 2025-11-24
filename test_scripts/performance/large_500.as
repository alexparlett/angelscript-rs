// Performance test file: ~500 lines of representative AngelScript code
// This file contains a mix of language features typical in game scripting:
// - Multiple classes with inheritance
// - Interfaces
// - Enums
// - Functions with various complexity
// - Control flow statements
// - Expression parsing
// - Templates and handles

// ============================================================================
// Core Data Types
// ============================================================================

enum EntityType {
    Player,
    Enemy,
    NPC,
    Item,
    Projectile,
    Trigger
}

enum DamageType {
    Physical,
    Fire,
    Ice,
    Lightning,
    Poison,
    Holy,
    Shadow
}

// ============================================================================
// Interfaces
// ============================================================================

interface IUpdatable {
    void update(float deltaTime);
}

interface IDamageable {
    void takeDamage(int amount, DamageType type);
    int getHealth() const;
    int getMaxHealth() const;
    bool isAlive() const;
}

interface IRenderable {
    void render();
    void setVisible(bool visible);
    bool isVisible() const;
}

// ============================================================================
// Base Classes
// ============================================================================

class Vector2 {
    float x;
    float y;

    Vector2() {
        x = 0.0;
        y = 0.0;
    }

    Vector2(float _x, float _y) {
        x = _x;
        y = _y;
    }

    float length() const {
        return sqrt(x * x + y * y);
    }

    void normalize() {
        float len = length();
        if (len > 0.0) {
            x /= len;
            y /= len;
        }
    }

    Vector2 opAdd(const Vector2& other) const {
        return Vector2(x + other.x, y + other.y);
    }

    Vector2 opSub(const Vector2& other) const {
        return Vector2(x - other.x, y - other.y);
    }

    Vector2 opMul(float scalar) const {
        return Vector2(x * scalar, y * scalar);
    }
}

class Transform {
    Vector2 position;
    float rotation;
    Vector2 scale;

    Transform() {
        position = Vector2();
        rotation = 0.0;
        scale = Vector2(1.0, 1.0);
    }

    void translate(float dx, float dy) {
        position.x += dx;
        position.y += dy;
    }

    void rotate(float angle) {
        rotation += angle;
        if (rotation > 360.0) rotation -= 360.0;
        if (rotation < 0.0) rotation += 360.0;
    }
}

// ============================================================================
// Entity System
// ============================================================================

abstract class Entity : IUpdatable, IRenderable {
    protected EntityType type;
    protected Transform transform;
    protected bool visible;
    protected bool active;
    protected string name;
    protected int id;

    Entity(EntityType _type, const string&in _name) {
        type = _type;
        name = _name;
        transform = Transform();
        visible = true;
        active = true;
        id = generateId();
    }

    void update(float deltaTime) {
        if (!active) return;
        onUpdate(deltaTime);
    }

    void render() {
        if (!visible || !active) return;
        onRender();
    }

    void setVisible(bool _visible) {
        visible = _visible;
    }

    bool isVisible() const {
        return visible;
    }

    EntityType getType() const {
        return type;
    }

    Vector2 getPosition() const {
        return transform.position;
    }

    void setPosition(const Vector2&in pos) {
        transform.position = pos;
    }

    protected void onUpdate(float deltaTime) {}
    protected void onRender() {}

    private int generateId() {
        return int(random() * 1000000);
    }
}

// ============================================================================
// Character System
// ============================================================================

class Character : Entity, IDamageable {
    protected int health;
    protected int maxHealth;
    protected float speed;
    protected int armor;
    protected array<DamageType> resistances;

    Character(const string&in _name, int _maxHealth) {
        super(EntityType::Player, _name);
        maxHealth = _maxHealth;
        health = maxHealth;
        speed = 5.0;
        armor = 0;
    }

    void takeDamage(int amount, DamageType type) {
        if (!isAlive()) return;

        int finalDamage = amount;

        // Apply armor reduction
        finalDamage = int(finalDamage * (100.0 / (100.0 + armor)));

        // Apply resistances
        for (uint i = 0; i < resistances.length(); i++) {
            if (resistances[i] == type) {
                finalDamage = int(finalDamage * 0.5);
                break;
            }
        }

        health -= finalDamage;
        if (health < 0) health = 0;

        onDamaged(finalDamage, type);
    }

    void heal(int amount) {
        if (!isAlive()) return;
        health += amount;
        if (health > maxHealth) {
            health = maxHealth;
        }
    }

    int getHealth() const {
        return health;
    }

    int getMaxHealth() const {
        return maxHealth;
    }

    bool isAlive() const {
        return health > 0;
    }

    void addResistance(DamageType type) {
        resistances.insertLast(type);
    }

    void move(float dx, float dy, float deltaTime) {
        transform.translate(dx * speed * deltaTime, dy * speed * deltaTime);
    }

    protected void onDamaged(int amount, DamageType type) {}
}

// ============================================================================
// Player Class
// ============================================================================

class Player : Character {
    private int experience;
    private int level;
    private int gold;
    private array<string> inventory;

    Player(const string&in _name) {
        super(_name, 100);
        experience = 0;
        level = 1;
        gold = 0;
        speed = 6.0;
    }

    void addExperience(int amount) {
        experience += amount;

        int requiredExp = level * 100;
        while (experience >= requiredExp) {
            levelUp();
            experience -= requiredExp;
            requiredExp = level * 100;
        }
    }

    void addGold(int amount) {
        gold += amount;
    }

    bool spendGold(int amount) {
        if (gold >= amount) {
            gold -= amount;
            return true;
        }
        return false;
    }

    void addItem(const string&in item) {
        inventory.insertLast(item);
    }

    bool hasItem(const string&in item) const {
        for (uint i = 0; i < inventory.length(); i++) {
            if (inventory[i] == item) {
                return true;
            }
        }
        return false;
    }

    private void levelUp() {
        level++;
        maxHealth += 10;
        health = maxHealth;
        armor += 5;
    }

    protected void onUpdate(float deltaTime) override {
        // Player-specific update logic
    }

    protected void onDamaged(int amount, DamageType type) override {
        // Play damage sound, show damage number, etc.
    }
}

// ============================================================================
// Enemy System
// ============================================================================

class Enemy : Character {
    private float aggroRange;
    private int expReward;
    private int goldReward;
    private Player@ target;

    Enemy(const string&in _name, int _health, int _expReward, int _goldReward) {
        super(_name, _health);
        type = EntityType::Enemy;
        expReward = _expReward;
        goldReward = _goldReward;
        aggroRange = 10.0;
        speed = 3.0;
        @target = null;
    }

    void setTarget(Player@ _target) {
        @target = _target;
    }

    protected void onUpdate(float deltaTime) override {
        if (target is null || !target.isAlive()) {
            return;
        }

        Vector2 targetPos = target.getPosition();
        Vector2 myPos = getPosition();
        Vector2 direction = targetPos - myPos;
        float distance = direction.length();

        if (distance <= aggroRange && distance > 1.0) {
            direction.normalize();
            move(direction.x, direction.y, deltaTime);
        } else if (distance <= 1.0) {
            attack(target);
        }
    }

    private void attack(IDamageable@ victim) {
        int damage = 10 + level * 2;
        victim.takeDamage(damage, DamageType::Physical);
    }

    protected void onDamaged(int amount, DamageType type) override {
        // Enemy AI reaction to damage
    }

    int getExpReward() const {
        return expReward;
    }

    int getGoldReward() const {
        return goldReward;
    }
}

// ============================================================================
// Projectile System
// ============================================================================

class Projectile : Entity {
    private Vector2 velocity;
    private int damage;
    private DamageType damageType;
    private float lifetime;
    private float maxLifetime;
    private Entity@ owner;

    Projectile(Entity@ _owner, const Vector2&in _velocity, int _damage, DamageType _type) {
        super(EntityType::Projectile, "projectile");
        @owner = _owner;
        velocity = _velocity;
        damage = _damage;
        damageType = _type;
        lifetime = 0.0;
        maxLifetime = 5.0;

        if (owner !is null) {
            setPosition(owner.getPosition());
        }
    }

    protected void onUpdate(float deltaTime) override {
        lifetime += deltaTime;

        if (lifetime >= maxLifetime) {
            active = false;
            return;
        }

        transform.translate(velocity.x * deltaTime, velocity.y * deltaTime);
    }

    bool checkCollision(Entity@ other) {
        if (other is null || other is owner) {
            return false;
        }

        Vector2 otherPos = other.getPosition();
        Vector2 myPos = getPosition();
        Vector2 diff = otherPos - myPos;

        return diff.length() < 1.0;
    }

    void onHit(IDamageable@ victim) {
        if (victim !is null) {
            victim.takeDamage(damage, damageType);
        }
        active = false;
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

float distance(const Vector2&in a, const Vector2&in b) {
    float dx = b.x - a.x;
    float dy = b.y - a.y;
    return sqrt(dx * dx + dy * dy);
}

float clamp(float value, float min, float max) {
    if (value < min) return min;
    if (value > max) return max;
    return value;
}

int randomRange(int min, int max) {
    return min + int(random() * (max - min));
}

bool chance(float probability) {
    return random() < probability;
}

// ============================================================================
// Game Manager
// ============================================================================

class GameManager {
    private Player@ player;
    private array<Enemy@> enemies;
    private array<Projectile@> projectiles;
    private int wave;
    private int score;

    GameManager() {
        @player = Player("Hero");
        wave = 1;
        score = 0;
    }

    void initialize() {
        spawnEnemyWave();
    }

    void update(float deltaTime) {
        if (player is null || !player.isAlive()) {
            return;
        }

        player.update(deltaTime);

        updateEnemies(deltaTime);
        updateProjectiles(deltaTime);
        checkCollisions();

        if (enemies.length() == 0) {
            nextWave();
        }
    }

    private void updateEnemies(float deltaTime) {
        for (uint i = 0; i < enemies.length(); i++) {
            enemies[i].setTarget(player);
            enemies[i].update(deltaTime);

            if (!enemies[i].isAlive()) {
                player.addExperience(enemies[i].getExpReward());
                player.addGold(enemies[i].getGoldReward());
                score += 100;
                enemies.removeAt(i);
                i--;
            }
        }
    }

    private void updateProjectiles(float deltaTime) {
        for (uint i = 0; i < projectiles.length(); i++) {
            projectiles[i].update(deltaTime);

            if (!projectiles[i].active) {
                projectiles.removeAt(i);
                i--;
            }
        }
    }

    private void checkCollisions() {
        for (uint i = 0; i < projectiles.length(); i++) {
            for (uint j = 0; j < enemies.length(); j++) {
                if (projectiles[i].checkCollision(enemies[j])) {
                    projectiles[i].onHit(enemies[j]);
                    break;
                }
            }
        }
    }

    private void spawnEnemyWave() {
        int enemyCount = wave * 3;
        for (int i = 0; i < enemyCount; i++) {
            Enemy@ enemy = Enemy(
                "Enemy" + i,
                50 + wave * 10,
                wave * 10,
                wave * 5
            );

            float angle = (i * 360.0) / enemyCount;
            float x = cos(angle) * 15.0;
            float y = sin(angle) * 15.0;
            enemy.setPosition(Vector2(x, y));

            enemies.insertLast(enemy);
        }
    }

    private void nextWave() {
        wave++;
        spawnEnemyWave();
    }

    void spawnProjectile(const Vector2&in direction, int damage, DamageType type) {
        Vector2 velocity = direction * 10.0;
        Projectile@ proj = Projectile(player, velocity, damage, type);
        projectiles.insertLast(proj);
    }
}
