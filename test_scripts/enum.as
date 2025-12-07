// Test enum declarations

enum Color {
    Red,
    Green,
    Blue
}

enum Difficulty {
    Easy = 0,
    Normal = 1,
    Hard = 2,
    Expert = 3
}

enum Flags {
    None = 0,
    Flag1 = 1,
    Flag2 = 2,
    Flag3 = 4,
    Flag4 = 8,
    All = 15
}

enum Status {
    Idle,
    Running,
    Paused,
    Stopped
}

void useEnums() {
    Color c = Color::Red;
    Difficulty d = Difficulty::Hard;
    Flags f = Flags::Flag1 | Flags::Flag2;
    Status s = Status::Running;
    
    if (c == Color::Red) {
        print("Red color");
    }
}
