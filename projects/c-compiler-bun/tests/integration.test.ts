/**
 * Integration tests
 */

import { describe, it, expect } from "bun:test";
import { Compiler } from "../src/compiler.js";

describe("Compiler Integration", () => {
  it("should compile a simple program", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        return 0;
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
    expect(result.ast).toBeDefined();
    expect(result.errors).toHaveLength(0);
  });

  it("should compile and run a simple program", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        return 42;
      }
    `;

    const result = compiler.compileAndRun(source);

    expect(result.success).toBe(true);
  });

  it("should handle variable declarations", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        int x = 10;
        int y = 20;
        return x + y;
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle if statements", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        int x = 10;
        if (x > 5) {
          return 1;
        } else {
          return 0;
        }
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle while loops", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        int x = 0;
        while (x < 10) {
          x = x + 1;
        }
        return x;
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle for loops", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        int sum = 0;
        for (int i = 0; i < 10; i = i + 1) {
          sum = sum + i;
        }
        return sum;
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle function definitions", () => {
    const compiler = new Compiler();
    const source = `
      int add(int a, int b) {
        return a + b;
      }

      int main() {
        return add(5, 3);
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle multiple functions", () => {
    const compiler = new Compiler();
    const source = `
      int square(int x) {
        return x * x;
      }

      int cube(int x) {
        return x * x * x;
      }

      int main() {
        return square(5);
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle array declarations", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        int arr[10];
        arr[0] = 42;
        return arr[0];
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle pointer operations", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        int x = 42;
        int* ptr = &x;
        return *ptr;
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should detect syntax errors", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        int x = 
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
  });

  it("should handle void functions", () => {
    const compiler = new Compiler();
    const source = `
      void print_hello() {
      }

      int main() {
        print_hello();
        return 0;
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle complex expressions", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        int x = (1 + 2) * (3 + 4);
        return x;
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle comparison operators", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        int x = 10;
        int y = 20;
        if (x < y) {
          return 1;
        }
        return 0;
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle else-if chains", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        int x = 5;
        if (x < 0) {
          return -1;
        } else if (x == 0) {
          return 0;
        } else {
          return 1;
        }
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle nested control structures", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        int sum = 0;
        for (int i = 0; i < 10; i = i + 1) {
          if (i % 2 == 0) {
            sum = sum + i;
          }
        }
        return sum;
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle float types", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        float x = 3.14;
        return 0;
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should handle char types", () => {
    const compiler = new Compiler();
    const source = `
      int main() {
        char c = 'A';
        return c;
      }
    `;

    const result = compiler.compile(source);

    expect(result.success).toBe(true);
  });

  it("should reset between compilations", () => {
    const compiler = new Compiler();
    const source1 = "int main() { return 1; }";
    const source2 = "int main() { return 2; }";

    const result1 = compiler.compile(source1);
    const result2 = compiler.compile(source2);

    expect(result1.success).toBe(true);
    expect(result2.success).toBe(true);
  });
});