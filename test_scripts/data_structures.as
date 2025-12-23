// Data structures example - demonstrates custom data structures

class Node {
    int value;
    Node@ next;
    
    Node(int val) {
        value = val;
        @next = null;
    }
}

class LinkedList {
    private Node@ head;
    private uint count;
    
    LinkedList() {
        @head = null;
        count = 0;
    }
    
    void insertFront(int value) {
        Node@ newNode = Node(value);
        @newNode.next = head;
        @head = newNode;
        count++;
    }
    
    void insertBack(int value) {
        Node@ newNode = Node(value);
        
        if (head is null) {
            @head = newNode;
        } else {
            Node@ current = head;
            while (current.next !is null) {
                @current = current.next;
            }
            @current.next = newNode;
        }
        count++;
    }
    
    bool remove(int value) {
        if (head is null) return false;
        
        if (head.value == value) {
            @head = head.next;
            count--;
            return true;
        }
        
        Node@ current = head;
        while (current.next !is null) {
            if (current.next.value == value) {
                @current.next = current.next.next;
                count--;
                return true;
            }
            @current = current.next;
        }
        
        return false;
    }
    
    bool contains(int value) const {
        Node@ current = head;
        while (current !is null) {
            if (current.value == value) return true;
            @current = current.next;
        }
        return false;
    }
    
    uint length() const {
        return count;
    }
    
    void printAll() const {
        Node@ current = head;
        while (current !is null) {
            print("{}", current.value);
            @current = current.next;
        }
    }
}

class Stack {
    private array<int> data;
    
    void push(int value) {
        data.insertLast(value);
    }
    
    int pop() {
        if (data.length() == 0) {
            return 0;  // Or throw error
        }
        int value = data[data.length() - 1];
        data.removeLast();
        return value;
    }
    
    int peek() const {
        if (data.length() == 0) {
            return 0;
        }
        return data[data.length() - 1];
    }
    
    bool isEmpty() const {
        return data.length() == 0;
    }
    
    uint size() const {
        return data.length();
    }
}

class Queue {
    private array<int> data;
    private uint frontIndex;
    
    Queue() {
        frontIndex = 0;
    }
    
    void enqueue(int value) {
        data.insertLast(value);
    }
    
    int dequeue() {
        if (isEmpty()) {
            return 0;
        }
        int value = data[frontIndex];
        frontIndex++;
        
        // Cleanup when queue gets too sparse
        if (frontIndex > 100 && frontIndex > data.length() / 2) {
            array<int> newData;
            for (uint i = frontIndex; i < data.length(); i++) {
                newData.insertLast(data[i]);
            }
            data = newData;
            frontIndex = 0;
        }
        
        return value;
    }
    
    int peek() const {
        if (isEmpty()) {
            return 0;
        }
        return data[frontIndex];
    }
    
    bool isEmpty() const {
        return frontIndex >= data.length();
    }
    
    uint size() const {
        if (frontIndex >= data.length()) return 0;
        return data.length() - frontIndex;
    }
}

class PriorityQueue {
    private array<int> heap;
    
    void insert(int value) {
        heap.insertLast(value);
        heapifyUp(heap.length() - 1);
    }
    
    int extractMin() {
        if (heap.length() == 0) return 0;
        
        int min = heap[0];
        heap[0] = heap[heap.length() - 1];
        heap.removeLast();
        
        if (heap.length() > 0) {
            heapifyDown(0);
        }
        
        return min;
    }
    
    private void heapifyUp(uint index) {
        while (index > 0) {
            uint parent = (index - 1) / 2;
            if (heap[index] >= heap[parent]) break;
            
            int temp = heap[index];
            heap[index] = heap[parent];
            heap[parent] = temp;
            
            index = parent;
        }
    }
    
    private void heapifyDown(uint index) {
        while (true) {
            uint left = 2 * index + 1;
            uint right = 2 * index + 2;
            uint smallest = index;
            
            if (left < heap.length() && heap[left] < heap[smallest]) {
                smallest = left;
            }
            if (right < heap.length() && heap[right] < heap[smallest]) {
                smallest = right;
            }
            
            if (smallest == index) break;
            
            int temp = heap[index];
            heap[index] = heap[smallest];
            heap[smallest] = temp;
            
            index = smallest;
        }
    }
    
    bool isEmpty() const {
        return heap.length() == 0;
    }
    
    uint size() const {
        return heap.length();
    }
}

void testDataStructures() {
    // Test linked list
    LinkedList list;
    list.insertFront(3);
    list.insertFront(2);
    list.insertFront(1);
    list.insertBack(4);
    list.printAll();
    
    // Test stack
    Stack stack;
    stack.push(1);
    stack.push(2);
    stack.push(3);
    int top = stack.pop();
    
    // Test queue
    Queue queue;
    queue.enqueue(1);
    queue.enqueue(2);
    queue.enqueue(3);
    int front = queue.dequeue();
    
    // Test priority queue
    PriorityQueue pq;
    pq.insert(5);
    pq.insert(2);
    pq.insert(8);
    pq.insert(1);
    int min = pq.extractMin();  // Should be 1
}
