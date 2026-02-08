/**
 * Test suite for TypeScript transpiler
 */

import { transpile } from "./index";

function runTest(name: string, input: string, expected: string): boolean {
  try {
    const result = transpile(input);
    const passed = result.trim() === expected.trim();

    if (passed) {
      console.log(`✓ ${name}`);
    } else {
      console.log(`✗ ${name}`);
      console.log("  Input:", input);
      console.log("  Expected:", expected);
      console.log("  Got:", result);
    }

    return passed;
  } catch (error) {
    console.log(`✗ ${name} (ERROR)`);
    console.log("  Error:", error instanceof Error ? error.message : error);
    return false;
  }
}

let passed = 0;
let total = 0;

function test(name: string, input: string, expected: string) {
  total++;
  if (runTest(name, input, expected)) {
    passed++;
  }
}

// Test 1: Simple variable declarations
test(
  "Variable declarations",
  `let x: number = 5;
  const y: string = "hello";
  var z: boolean = true;`,
  `let x = 5;
  const y = "hello";
  var z = true;`
);

// Test 2: Function declarations
test(
  "Function declaration",
  `function add(a: number, b: number): number {
    return a + b;
  }`,
  `function add(a, b) {
    return a + b;
  }`
);

// Test 3: Arrow functions
test(
  "Arrow function",
  `const multiply = (a: number, b: number): number => a * b;`,
  `const multiply = (a, b) => a * b;`
);

// Test 4: Class declarations
test(
  "Class declaration",
  `class Person {
    name: string;
    age: number;

    constructor(name: string, age: number) {
      this.name = name;
      this.age = age;
    }

    greet(): string {
      return "Hello, " + this.name;
    }
  }`,
  `class Person {
    name;
    age;

    constructor(name, age) {
      this.name = name;
      this.age = age;
    }

    greet() {
      return "Hello, " + this.name;
    }
  }`
);

// Test 5: Interface (should be removed)
test(
  "Interface declaration (removed)",
  `interface User {
    name: string;
    age: number;
  }

  const user: User = { name: "John", age: 30 };`,
  `const user = { name: "John", age: 30 };`
);

// Test 6: Type alias (should be removed)
test(
  "Type alias (removed)",
  `type ID = string | number;
  const id: ID = "abc123";`,
  `const id = "abc123";`
);

// Test 7: Enum
test(
  "Enum declaration",
  `enum Color {
    Red,
    Green,
    Blue
  }

  const c: Color = Color.Red;`,
  `var Color;
  (function (Color) {
    Color[Color["Red"] = 0] = "Red";
    Color[Color["Green"] = 1] = "Green";
    Color[Color["Blue"] = 2] = "Blue";
  })(Color || (Color = {}));
  const c = Color.Red;`
);

// Test 8: Async function
test(
  "Async function",
  `async function fetchData(url: string): Promise<any> {
    const response = await fetch(url);
    return response.json();
  }`,
  `async function fetchData(url) {
    const response = await fetch(url);
    return response.json();
  }`
);

// Test 9: Import/Export
test(
  "Import/Export",
  `import { foo, bar } from "./module";
  export const baz = 42;`,
  `import { foo, bar } from "./module";
  export const baz = 42;`
);

// Test 10: Generic function (type parameters removed)
test(
  "Generic function",
  `function identity<T>(arg: T): T {
    return arg;
  }`,
  `function identity(arg) {
    return arg;
  }`
);

// Test 11: Optional parameters
test(
  "Optional parameters",
  `function greet(name: string, greeting?: string): string {
    return greeting ? greeting + ", " + name : "Hello, " + name;
  }`,
  `function greet(name, greeting) {
    return greeting ? greeting + ", " + name : "Hello, " + name;
  }`
);

// Test 12: Rest parameters
test(
  "Rest parameters",
  `function sum(...numbers: number[]): number {
    return numbers.reduce((a, b) => a + b, 0);
  }`,
  `function sum(...numbers) {
    return numbers.reduce((a, b) => a + b, 0);
  }`
);

// Test 13: Class with static and private members
test(
  "Class with modifiers",
  `class Counter {
    private count: number = 0;
    static instance: Counter;

    increment(): void {
      this.count++;
    }

    static getInstance(): Counter {
      if (!Counter.instance) {
        Counter.instance = new Counter();
      }
      return Counter.instance;
    }
  }`,
  `class Counter {
    count = 0;
    static instance;

    increment() {
      this.count++;
    }

    static getInstance() {
      if (!Counter.instance) {
        Counter.instance = new Counter();
      }
      return Counter.instance;
    }
  }`
);

// Test 14: Array and object literals
test(
  "Array and object literals",
  `const arr: number[] = [1, 2, 3];
  const obj: { x: number; y: number } = { x: 10, y: 20 };`,
  `const arr = [1, 2, 3];
  const obj = { x: 10, y: 20 };`
);

// Test 15: If/else statement
test(
  "If/else statement",
  `function max(a: number, b: number): number {
    if (a > b) {
      return a;
    } else {
      return b;
    }
  }`,
  `function max(a, b) {
    if (a > b) {
      return a;
    } else {
      return b;
    }
  }`
);

// Test 16: For loop
test(
  "For loop",
  `function sumArray(arr: number[]): number {
    let sum = 0;
    for (let i = 0; i < arr.length; i++) {
      sum += arr[i];
    }
    return sum;
  }`,
  `function sumArray(arr) {
    let sum = 0;
    for (let i = 0; i < arr.length; i++) {
      sum += arr[i];
    }
    return sum;
  }`
);

// Test 17: While loop
test(
  "While loop",
  `function countdown(n: number): void {
    while (n > 0) {
      console.log(n);
      n--;
    }
  }`,
  `function countdown(n) {
    while (n > 0) {
      console.log(n);
      n--;
    }
  }`
);

// Test 18: Method with access modifiers
test(
  "Class with method modifiers",
  `class BankAccount {
    private balance: number;

    constructor(initial: number) {
      this.balance = initial;
    }

    public deposit(amount: number): void {
      this.balance += amount;
    }

    protected getBalance(): number {
      return this.balance;
    }
  }`,
  `class BankAccount {
    balance;

    constructor(initial) {
      this.balance = initial;
    }

    deposit(amount) {
      this.balance += amount;
    }

    getBalance() {
      return this.balance;
    }
  }`
);

// Test 19: Constructor parameter properties
test(
  "Constructor parameter properties",
  `class Point {
    constructor(public x: number, public y: number) {}

    toString(): string {
      return \`(\${this.x}, \${this.y})\`;
    }
  }`,
  `class Point {
    constructor(x, y) {
    }

    toString() {
      return "(" + this.x + ", " + this.y + ")";
    }
  }`
);

// Test 20: Complex expression
test(
  "Complex expression",
  `const result: number = (a + b) * c / (d - e);`,
  `const result = (a + b) * c / (d - e);`
);

// Print summary
console.log("\n" + "=".repeat(50));
console.log(`Tests passed: ${passed}/${total}`);
console.log("=".repeat(50));

if (passed === total) {
  console.log("\n✓ All tests passed!");
  process.exit(0);
} else {
  console.log(`\n✗ ${total - passed} test(s) failed`);
  process.exit(1);
}