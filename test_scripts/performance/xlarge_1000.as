// Performance test file: ~1000 lines of complex AngelScript code
// This file extends large_500.as with additional systems and complexity:
// - More namespaces and organization
// - Complex inheritance hierarchies
// - Template usage
// - Advanced game systems (inventory, skills, AI)
// - State machines
// - More interfaces and abstract classes

// FFI placeholders - will be replaced with proper FFI bindings
float sqrt(float x) { return x; }
float sin(float x) { return x; }
float cos(float x) { return x; }
float random() { return 0.5; }
void print(const string &in msg) {}

// ============================================================================
// Core Types and Enums
// ============================================================================

enum EntityType {
    Player, Enemy, NPC, Item, Projectile, Trigger, Obstacle, Decoration
}

enum DamageType {
    Physical, Fire, Ice, Lightning, Poison, Holy, Shadow, Arcane
}

enum ItemType {
    Weapon, Armor, Consumable, QuestItem, Material, Key
}

enum ItemRarity {
    Common, Uncommon, Rare, Epic, Legendary, Mythic
}

enum SkillType {
    Active, Passive, Ultimate, Aura
}

enum AIState {
    Idle, Patrol, Chase, Attack, Flee, Dead
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

interface IInteractable {
    void interact(Player@ player);
    string getInteractionText() const;
    bool canInteract(Player@ player) const;
}

interface ICollectable {
    void onCollected(Player@ player);
    ItemType getItemType() const;
    ItemRarity getRarity() const;
}

// ============================================================================
// Math Utilities
// ============================================================================

namespace Math {
    const float PI = 3.14159265359;
    const float TAU = 6.28318530718;

    float lerp(float a, float b, float t) {
        return a + (b - a) * t;
    }

    float clamp(float value, float min, float max) {
        if (value < min) return min;
        if (value > max) return max;
        return value;
    }

    int clampi(int value, int min, int max) {
        if (value < min) return min;
        if (value > max) return max;
        return value;
    }

    float smoothstep(float edge0, float edge1, float x) {
        float t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
        return t * t * (3.0 - 2.0 * t);
    }
}

// ============================================================================
// Vector Classes
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

    float lengthSquared() const {
        return x * x + y * y;
    }

    void normalize() {
        float len = length();
        if (len > 0.001) {
            x /= len;
            y /= len;
        }
    }

    Vector2 normalized() const {
        Vector2 result(x, y);
        result.normalize();
        return result;
    }

    float dot(const Vector2&in other) const {
        return x * other.x + y * other.y;
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

    Vector2 opDiv(float scalar) const {
        return Vector2(x / scalar, y / scalar);
    }
}

class Rect {
    float x;
    float y;
    float width;
    float height;

    Rect() {
        x = 0.0;
        y = 0.0;
        width = 0.0;
        height = 0.0;
    }

    Rect(float _x, float _y, float _w, float _h) {
        x = _x;
        y = _y;
        width = _w;
        height = _h;
    }

    bool contains(const Vector2&in point) const {
        return point.x >= x && point.x <= x + width &&
               point.y >= y && point.y <= y + height;
    }

    bool intersects(const Rect&in other) const {
        return x < other.x + other.width &&
               x + width > other.x &&
               y < other.y + other.height &&
               y + height > other.y;
    }
}

// ============================================================================
// Transform and Component System
// ============================================================================

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
        while (rotation > Math::TAU) rotation -= Math::TAU;
        while (rotation < 0.0) rotation += Math::TAU;
    }

    Vector2 forward() const {
        return Vector2(cos(rotation), sin(rotation));
    }
}

// ============================================================================
// Stats and Attributes
// ============================================================================

class Stats {
    int strength;
    int agility;
    int intelligence;
    int vitality;
    int luck;

    Stats() {
        strength = 10;
        agility = 10;
        intelligence = 10;
        vitality = 10;
        luck = 10;
    }

    Stats(int str, int agi, int intel, int vit, int lck) {
        strength = str;
        agility = agi;
        intelligence = intel;
        vitality = vit;
        luck = lck;
    }

    int getTotalStats() const {
        return strength + agility + intelligence + vitality + luck;
    }

    void addStats(const Stats&in other) {
        strength += other.strength;
        agility += other.agility;
        intelligence += other.intelligence;
        vitality += other.vitality;
        luck += other.luck;
    }
}

// ============================================================================
// Entity Base Class
// ============================================================================

abstract class Entity : IUpdatable, IRenderable {
    protected EntityType type;
    protected Transform transform;
    protected bool visible;
    protected bool active;
    protected string name;
    protected int id;
    protected Rect bounds;

    Entity(EntityType _type, const string&in _name) {
        type = _type;
        name = _name;
        transform = Transform();
        visible = true;
        active = true;
        id = generateId();
        bounds = Rect(0, 0, 1, 1);
    }

    void update(float deltaTime) {
        if (!active) return;
        onUpdate(deltaTime);
        updateBounds();
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

    Rect getBounds() const {
        return bounds;
    }

    bool checkCollision(Entity@ other) {
        if (other is null) return false;
        return bounds.intersects(other.getBounds());
    }

    protected void onUpdate(float deltaTime) {}
    protected void onRender() {}

    protected void updateBounds() {
        bounds.x = transform.position.x - bounds.width * 0.5;
        bounds.y = transform.position.y - bounds.height * 0.5;
    }

    private int generateId() {
        return int(random() * 10000000);
    }
}

// ============================================================================
// Character System
// ============================================================================

abstract class Character : Entity, IDamageable {
    protected int health;
    protected int maxHealth;
    protected int mana;
    protected int maxMana;
    protected float speed;
    protected int armor;
    protected int magicResist;
    protected Stats stats;
    protected array<DamageType> resistances;
    protected array<DamageType> weaknesses;

    Character(EntityType _type, const string&in _name, int _maxHealth, int _maxMana) {
        super(_type, _name);
        maxHealth = _maxHealth;
        health = maxHealth;
        maxMana = _maxMana;
        mana = maxMana;
        speed = 5.0;
        armor = 0;
        magicResist = 0;
        stats = Stats();
    }

    void takeDamage(int amount, DamageType type) {
        if (!isAlive()) return;

        int finalDamage = calculateDamage(amount, type);
        health -= finalDamage;
        if (health < 0) health = 0;

        onDamaged(finalDamage, type);

        if (!isAlive()) {
            onDeath();
        }
    }

    void heal(int amount) {
        if (!isAlive()) return;
        health += amount;
        if (health > maxHealth) {
            health = maxHealth;
        }
    }

    void restoreMana(int amount) {
        mana += amount;
        if (mana > maxMana) {
            mana = maxMana;
        }
    }

    bool consumeMana(int amount) {
        if (mana >= amount) {
            mana -= amount;
            return true;
        }
        return false;
    }

    int getHealth() const {
        return health;
    }

    int getMaxHealth() const {
        return maxHealth;
    }

    int getMaxMana() const {
        return maxMana;
    }

    bool isAlive() const {
        return health > 0;
    }

    void addResistance(DamageType type) {
        resistances.insertLast(type);
    }

    void addWeakness(DamageType type) {
        weaknesses.insertLast(type);
    }

    void move(const Vector2&in direction, float deltaTime) {
        if (direction.lengthSquared() > 0.001) {
            Vector2 normalized = direction.normalized();
            transform.translate(
                normalized.x * speed * deltaTime,
                normalized.y * speed * deltaTime
            );
        }
    }

    protected int calculateDamage(int amount, DamageType type) {
        int finalDamage = amount;

        // Apply armor for physical damage
        if (type == DamageType::Physical) {
            float reduction = armor / (armor + 100.0);
            finalDamage = int(finalDamage * (1.0 - reduction));
        } else {
            // Apply magic resist for magical damage
            float reduction = magicResist / (magicResist + 100.0);
            finalDamage = int(finalDamage * (1.0 - reduction));
        }

        // Check for resistances (50% reduction)
        for (uint i = 0; i < resistances.length(); i++) {
            if (resistances[i] == type) {
                finalDamage = int(finalDamage * 0.5);
                break;
            }
        }

        // Check for weaknesses (150% damage)
        for (uint i = 0; i < weaknesses.length(); i++) {
            if (weaknesses[i] == type) {
                finalDamage = int(finalDamage * 1.5);
                break;
            }
        }

        return Math::clampi(finalDamage, 1, 999999);
    }

    protected void onDamaged(int amount, DamageType type) {}
    protected void onDeath() {}
}

// ============================================================================
// Skill System
// ============================================================================

class Skill {
    string name;
    string description;
    SkillType type;
    int manaCost;
    float cooldown;
    float currentCooldown;
    int damage;
    DamageType damageType;
    float range;

    Skill(const string&in _name, SkillType _type, int _manaCost, float _cooldown) {
        name = _name;
        type = _type;
        manaCost = _manaCost;
        cooldown = _cooldown;
        currentCooldown = 0.0;
        damage = 0;
        damageType = DamageType::Physical;
        range = 5.0;
    }

    void update(float deltaTime) {
        if (currentCooldown > 0.0) {
            currentCooldown -= deltaTime;
            if (currentCooldown < 0.0) {
                currentCooldown = 0.0;
            }
        }
    }

    bool canUse() const {
        return currentCooldown <= 0.0;
    }

    void use() {
        currentCooldown = cooldown;
    }

    float getCooldownPercent() const {
        if (cooldown <= 0.0) return 0.0;
        return Math::clamp(currentCooldown / cooldown, 0.0, 1.0);
    }
}

// ============================================================================
// Inventory System
// ============================================================================

class Item {
    string name;
    string description;
    ItemType type;
    ItemRarity rarity;
    int value;
    int stackSize;
    bool consumable;

    Item(const string&in _name, ItemType _type, ItemRarity _rarity) {
        name = _name;
        type = _type;
        rarity = _rarity;
        value = 0;
        stackSize = 1;
        consumable = false;
    }

    int getSellValue() const {
        return int(value * 0.5);
    }
}

class InventorySlot {
    Item@ item;
    int quantity;

    InventorySlot() {
        @item = null;
        quantity = 0;
    }

    bool isEmpty() const {
        return item is null || quantity <= 0;
    }

    bool isFull() const {
        if (item is null) return false;
        return quantity >= item.stackSize;
    }

    bool canStack(Item@ other) const {
        if (item is null || other is null) return false;
        return item.name == other.name && !isFull();
    }
}

class Inventory {
    private array<InventorySlot@> slots;
    private int maxSlots;

    Inventory(int _maxSlots) {
        maxSlots = _maxSlots;
        for (int i = 0; i < maxSlots; i++) {
            slots.insertLast(InventorySlot());
        }
    }

    bool addItem(Item@ item, int quantity = 1) {
        if (item is null) return false;

        // Try to stack with existing items
        for (uint i = 0; i < slots.length(); i++) {
            if (slots[i].canStack(item)) {
                int spaceLeft = item.stackSize - slots[i].quantity;
                int toAdd = Math::clampi(quantity, 0, spaceLeft);
                slots[i].quantity += toAdd;
                quantity -= toAdd;

                if (quantity <= 0) return true;
            }
        }

        // Find empty slot
        for (uint i = 0; i < slots.length(); i++) {
            if (slots[i].isEmpty()) {
                @slots[i].item = item;
                slots[i].quantity = Math::clampi(quantity, 0, item.stackSize);
                quantity -= slots[i].quantity;

                if (quantity <= 0) return true;
            }
        }

        return quantity <= 0;
    }

    bool removeItem(const string&in itemName, int quantity = 1) {
        int remaining = quantity;

        for (uint i = 0; i < slots.length(); i++) {
            if (!slots[i].isEmpty() && slots[i].item.name == itemName) {
                int toRemove = Math::clampi(remaining, 0, slots[i].quantity);
                slots[i].quantity -= toRemove;
                remaining -= toRemove;

                if (slots[i].quantity <= 0) {
                    @slots[i].item = null;
                }

                if (remaining <= 0) return true;
            }
        }

        return remaining <= 0;
    }

    bool hasItem(const string&in itemName, int quantity = 1) const {
        int count = 0;
        for (uint i = 0; i < slots.length(); i++) {
            if (!slots[i].isEmpty() && slots[i].item.name == itemName) {
                count += slots[i].quantity;
                if (count >= quantity) return true;
            }
        }
        return false;
    }

    int getItemCount(const string&in itemName) const {
        int count = 0;
        for (uint i = 0; i < slots.length(); i++) {
            if (!slots[i].isEmpty() && slots[i].item.name == itemName) {
                count += slots[i].quantity;
            }
        }
        return count;
    }
}

// ============================================================================
// Player Class
// ============================================================================

class Player : Character {
    private int experience;
    private int level;
    private int gold;
    private Inventory@ inventory;
    private array<Skill@> skills;
    private int skillPoints;

    Player(const string&in _name) {
        super(EntityType::Player, _name, 100, 50);
        experience = 0;
        level = 1;
        gold = 0;
        skillPoints = 0;
        speed = 6.0;
        @inventory = Inventory(20);

        initializeSkills();
    }

    void addExperience(int amount) {
        experience += amount;

        int requiredExp = getRequiredExp();
        while (experience >= requiredExp) {
            levelUp();
            experience -= requiredExp;
            requiredExp = getRequiredExp();
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

    bool useSkill(int skillIndex, Entity@ target) {
        if (skillIndex < 0 || skillIndex >= int(skills.length())) {
            return false;
        }

        Skill@ skill = skills[skillIndex];
        if (!skill.canUse() || !consumeMana(skill.manaCost)) {
            return false;
        }

        skill.use();

        if (target !is null) {
            IDamageable@ damageable = cast<IDamageable>(target);
            if (damageable !is null) {
                damageable.takeDamage(skill.damage, skill.damageType);
            }
        }

        return true;
    }

    Inventory@ getInventory() {
        return inventory;
    }

    private void initializeSkills() {
        Skill@ fireball = Skill("Fireball", SkillType::Active, 20, 3.0);
        fireball.damage = 30;
        fireball.damageType = DamageType::Fire;
        skills.insertLast(fireball);

        Skill@ heal = Skill("Heal", SkillType::Active, 15, 5.0);
        heal.damage = -25;
        skills.insertLast(heal);

        Skill@ shield = Skill("Shield", SkillType::Active, 10, 8.0);
        skills.insertLast(shield);
    }

    private int getRequiredExp() const {
        return level * level * 100;
    }

    private void levelUp() {
        level++;
        skillPoints++;
        maxHealth += 15;
        health = maxHealth;
        maxMana += 10;
        mana = maxMana;
        armor += 3;

        stats.strength += 2;
        stats.agility += 2;
        stats.intelligence += 2;
        stats.vitality += 2;
        stats.luck += 1;
    }

    protected void onUpdate(float deltaTime) override {
        for (uint i = 0; i < skills.length(); i++) {
            skills[i].update(deltaTime);
        }
    }

    protected void onDamaged(int amount, DamageType type) override {
        // Play damage effects
    }

    protected void onDeath() override {
        // Handle player death
    }
}

// ============================================================================
// Enemy AI System
// ============================================================================

class EnemyAI {
    private AIState state;
    private Entity@ target;
    private float stateTimer;
    private Vector2 patrolPoint;

    EnemyAI() {
        state = AIState::Idle;
        @target = null;
        stateTimer = 0.0;
        patrolPoint = Vector2();
    }

    void update(Enemy@ self, float deltaTime) {
        stateTimer += deltaTime;

        switch (state) {
            case AIState::Idle:
                updateIdle(self, deltaTime);
                break;
            case AIState::Patrol:
                updatePatrol(self, deltaTime);
                break;
            case AIState::Chase:
                updateChase(self, deltaTime);
                break;
            case AIState::Attack:
                updateAttack(self, deltaTime);
                break;
            case AIState::Flee:
                updateFlee(self, deltaTime);
                break;
        }
    }

    void setTarget(Entity@ _target) {
        @target = _target;
    }

    AIState getState() const {
        return state;
    }

    private void updateIdle(Enemy@ self, float deltaTime) {
        if (stateTimer > 2.0) {
            changeState(AIState::Patrol);
        }

        if (target !is null) {
            float dist = (target.getPosition() - self.getPosition()).length();
            if (dist < 10.0) {
                changeState(AIState::Chase);
            }
        }
    }

    private void updatePatrol(Enemy@ self, float deltaTime) {
        Vector2 toPoint = patrolPoint - self.getPosition();
        if (toPoint.length() < 1.0) {
            changeState(AIState::Idle);
            return;
        }

        self.move(toPoint.normalized(), deltaTime);

        if (target !is null) {
            float dist = (target.getPosition() - self.getPosition()).length();
            if (dist < 10.0) {
                changeState(AIState::Chase);
            }
        }
    }

    private void updateChase(Enemy@ self, float deltaTime) {
        if (target is null) {
            changeState(AIState::Idle);
            return;
        }

        Vector2 toTarget = target.getPosition() - self.getPosition();
        float dist = toTarget.length();

        if (dist > 15.0) {
            changeState(AIState::Patrol);
            return;
        }

        if (dist <= 2.0) {
            changeState(AIState::Attack);
            return;
        }

        self.move(toTarget.normalized(), deltaTime);
    }

    private void updateAttack(Enemy@ self, float deltaTime) {
        if (target is null) {
            changeState(AIState::Idle);
            return;
        }

        float dist = (target.getPosition() - self.getPosition()).length();
        if (dist > 2.5) {
            changeState(AIState::Chase);
            return;
        }

        if (stateTimer > 1.0) {
            self.performAttack(target);
            stateTimer = 0.0;
        }
    }

    private void updateFlee(Enemy@ self, float deltaTime) {
        if (target is null) {
            changeState(AIState::Idle);
            return;
        }

        Vector2 awayFromTarget = self.getPosition() - target.getPosition();
        self.move(awayFromTarget.normalized(), deltaTime);

        if (stateTimer > 3.0) {
            changeState(AIState::Idle);
        }
    }

    private void changeState(AIState newState) {
        state = newState;
        stateTimer = 0.0;

        if (state == AIState::Patrol) {
            patrolPoint = Vector2(
                random() * 20.0 - 10.0,
                random() * 20.0 - 10.0
            );
        }
    }
}

// ============================================================================
// Enemy Class
// ============================================================================

class Enemy : Character {
    private float aggroRange;
    private int expReward;
    private int goldReward;
    private EnemyAI@ ai;
    private float attackCooldown;
    private float attackTimer;

    Enemy(const string&in _name, int _health, int _expReward, int _goldReward) {
        super(EntityType::Enemy, _name, _health, 0);
        expReward = _expReward;
        goldReward = _goldReward;
        aggroRange = 10.0;
        speed = 4.0;
        attackCooldown = 1.5;
        attackTimer = 0.0;
        @ai = EnemyAI();
    }

    void setTarget(Entity@ target) {
        ai.setTarget(target);
    }

    void performAttack(Entity@ target) {
        if (attackTimer > 0.0) return;

        IDamageable@ damageable = cast<IDamageable>(target);
        if (damageable !is null) {
            int damage = 10 + stats.strength * 2;
            damageable.takeDamage(damage, DamageType::Physical);
            attackTimer = attackCooldown;
        }
    }

    int getExpReward() const {
        return expReward;
    }

    int getGoldReward() const {
        return goldReward;
    }

    protected void onUpdate(float deltaTime) override {
        if (attackTimer > 0.0) {
            attackTimer -= deltaTime;
        }

        ai.update(this, deltaTime);
    }

    protected void onDamaged(int amount, DamageType type) override {
        // React to damage - maybe become more aggressive
    }

    protected void onDeath() override {
        // Death animation, drop loot, etc.
    }
}

// ============================================================================
// Game World and Manager
// ============================================================================

class GameWorld {
    private Player@ player;
    private array<Enemy@> enemies;
    private array<Item@> items;
    private int wave;
    private int score;
    private float gameTime;

    GameWorld() {
        @player = Player("Hero");
        wave = 1;
        score = 0;
        gameTime = 0.0;
    }

    void initialize() {
        spawnEnemyWave();
        createItems();
    }

    void update(float deltaTime) {
        gameTime += deltaTime;

        if (player is null || !player.isAlive()) {
            return;
        }

        player.update(deltaTime);
        updateEnemies(deltaTime);

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
                score += 100 * wave;
                enemies.removeAt(i);
                i--;
            }
        }
    }

    private void spawnEnemyWave() {
        int enemyCount = 3 + wave * 2;
        for (int i = 0; i < enemyCount; i++) {
            Enemy@ enemy = Enemy(
                "Enemy_W" + wave + "_" + i,
                30 + wave * 20,
                wave * 15,
                wave * 10
            );

            float angle = (i * Math::TAU) / enemyCount;
            float radius = 15.0 + wave * 2.0;
            enemy.setPosition(Vector2(
                cos(angle) * radius,
                sin(angle) * radius
            ));

            enemies.insertLast(enemy);
        }
    }

    private void createItems() {
        Item@ potion = Item("Health Potion", ItemType::Consumable, ItemRarity::Common);
        potion.value = 25;
        potion.stackSize = 99;
        items.insertLast(potion);

        Item@ sword = Item("Iron Sword", ItemType::Weapon, ItemRarity::Common);
        sword.value = 100;
        items.insertLast(sword);
    }

    private void nextWave() {
        wave++;
        spawnEnemyWave();
        player.restoreMana(player.getMaxMana());
    }
}
