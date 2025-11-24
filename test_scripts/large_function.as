// Large function - tests parser performance with complex functions

void largeFunction() {
    int x = 0;
    
    if (x == 0) { x++; } else if (x == 1) { x++; } else { x--; }
    if (x == 0) { x++; } else if (x == 1) { x++; } else { x--; }
    if (x == 0) { x++; } else if (x == 1) { x++; } else { x--; }
    if (x == 0) { x++; } else if (x == 1) { x++; } else { x--; }
    if (x == 0) { x++; } else if (x == 1) { x++; } else { x--; }
    
    for (int i = 0; i < 10; i++) { x += i; }
    for (int i = 0; i < 10; i++) { x += i; }
    for (int i = 0; i < 10; i++) { x += i; }
    for (int i = 0; i < 10; i++) { x += i; }
    for (int i = 0; i < 10; i++) { x += i; }
    
    while (x < 100) { x++; }
    while (x > 50) { x--; }
    while (x < 75) { x++; }
    
    switch (x) {
        case 0: x = 1; break;
        case 1: x = 2; break;
        case 2: x = 3; break;
        case 3: x = 4; break;
        case 4: x = 5; break;
        case 5: x = 6; break;
        case 6: x = 7; break;
        case 7: x = 8; break;
        case 8: x = 9; break;
        case 9: x = 10; break;
        default: x = 0; break;
    }
    
    int a = 1, b = 2, c = 3, d = 4, e = 5;
    int result1 = a + b * c - d / e;
    int result2 = (a + b) * (c - d) / e;
    int result3 = a * b + c * d + e;
    int result4 = (a * b * c) + (d * e);
    int result5 = ((a + b) * c) - ((d + e) / 2);
    
    for (int i = 0; i < 5; i++) {
        for (int j = 0; j < 5; j++) {
            for (int k = 0; k < 5; k++) {
                x += i * j * k;
            }
        }
    }
    
    if (x > 0) {
        if (x > 10) {
            if (x > 20) {
                if (x > 30) {
                    if (x > 40) {
                        x = 100;
                    }
                }
            }
        }
    }
    
    array<int> arr = {0, 1, 2, 3, 4, 5, 6, 7, 8, 9};
    arr[0] = 0; arr[1] = 1; arr[2] = 2; arr[3] = 3; arr[4] = 4;
    arr[5] = 5; arr[6] = 6; arr[7] = 7; arr[8] = 8; arr[9] = 9;
    
    for (int i = 0; i < 10; i++) {
        x += arr[i];
    }
    
    do {
        x--;
    } while (x > 0 && x < 100);
    
    bool flag1 = (x > 0 && x < 100) || (x > 200 && x < 300);
    bool flag2 = (x % 2 == 0) && (x % 3 == 0);
    bool flag3 = !(x == 0 || x == 1);
    bool flag4 = (x & 0xFF) == 0xFF;
    bool flag5 = ((x >> 4) & 0x0F) == 0x0F;
    
    if (flag1 && flag2 && flag3 && flag4 && flag5) {
        x = 0;
    }
    
    for (int i = 0; i < 20; i++) {
        if (i % 2 == 0) continue;
        if (i > 15) break;
        x += i;
    }
}
