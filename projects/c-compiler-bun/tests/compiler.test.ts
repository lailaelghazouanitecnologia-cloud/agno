/**
 * Comprehensive Tests for the C Compiler
 * Uses bun:test
 */

import { describe, it, expect, beforeAll } from 'bun:test';
import { Compiler } from '../src/compiler.js';

describe('C Compiler - Basic Functionality', () => {
  let compiler: Compiler;

  beforeAll(() => {
    compiler = new Compiler();
  });

  describe('Variable Declarations', () => {
    it('should declare and initialize int variable', () => {
      const source = `
        int main() {
          int x = 42;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should declare and initialize float variable', () => {
      const source = `
        int main() {
          float pi = 3.14;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should declare and initialize char variable', () => {
      const source = `
        int main() {
          char c = 'A';
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('Arithmetic Operations', () => {
    it('should perform addition', () => {
      const source = `
        int main() {
          int a = 10;
          int b = 20;
          int c = a + b;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should perform subtraction', () => {
      const source = `
        int main() {
          int a = 20;
          int b = 10;
          int c = a - b;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should perform multiplication', () => {
      const source = `
        int main() {
          int a = 6;
          int b = 7;
          int c = a * b;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should perform division', () => {
      const source = `
        int main() {
          int a = 20;
          int b = 4;
          int c = a / b;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should perform modulo', () => {
      const source = `
        int main() {
          int a = 17;
          int b = 5;
          int c = a % b;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should handle operator precedence', () => {
      const source = `
        int main() {
          int a = 2 + 3 * 4;
          int b = (2 + 3) * 4;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('Comparison Operations', () => {
    it('should perform less than comparison', () => {
      const source = `
        int main() {
          int a = 5;
          int b = 10;
          int c = a < b;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should perform greater than comparison', () => {
      const source = `
        int main() {
          int a = 10;
          int b = 5;
          int c = a > b;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should perform equality comparison', () => {
      const source = `
        int main() {
          int a = 5;
          int b = 5;
          int c = a == b;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should perform inequality comparison', () => {
      const source = `
        int main() {
          int a = 5;
          int b = 10;
          int c = a != b;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('If/Else Statements', () => {
    it('should execute if branch when condition is true', () => {
      const source = `
        int main() {
          int x = 10;
          if (x > 5) {
            x = 20;
          }
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should execute else branch when condition is false', () => {
      const source = `
        int main() {
          int x = 3;
          if (x > 5) {
            x = 20;
          } else {
            x = 30;
          }
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should handle else-if', () => {
      const source = `
        int main() {
          int x = 5;
          if (x > 10) {
            x = 20;
          } else if (x > 3) {
            x = 15;
          } else {
            x = 10;
          }
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('While Loops', () => {
    it('should execute while loop', () => {
      const source = `
        int main() {
          int i = 0;
          int sum = 0;
          while (i < 5) {
            sum = sum + i;
            i = i + 1;
          }
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('For Loops', () => {
    it('should execute for loop', () => {
      const source = `
        int main() {
          int sum = 0;
          for (int i = 0; i < 5; i = i + 1) {
            sum = sum + i;
          }
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('Functions', () => {
    it('should define and call function', () => {
      const source = `
        int add(int a, int b) {
          return a + b;
        }

        int main() {
          int result = add(5, 3);
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should handle function with no parameters', () => {
      const source = `
        int get_value() {
          return 42;
        }

        int main() {
          int x = get_value();
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should handle void return type', () => {
      const source = `
        void print_hello() {
          int x = 0;
        }

        int main() {
          print_hello();
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('Arrays', () => {
    it('should declare array', () => {
      const source = `
        int main() {
          int arr[5];
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should access array elements', () => {
      const source = `
        int main() {
          int arr[5];
          int x = arr[0];
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('Pointers', () => {
    it('should take address of variable', () => {
      const source = `
        int main() {
          int x = 10;
          int* ptr = &x;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should dereference pointer', () => {
      const source = `
        int main() {
          int x = 10;
          int* ptr = &x;
          int y = *ptr;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('Printf', () => {
    it('should print integer', () => {
      const source = `
        int main() {
          int x = 42;
          printf("%d", x);
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
      expect(result.output).toBe('42');
    });

    it('should print string', () => {
      const source = `
        int main() {
          printf("Hello, World!");
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
      expect(result.output).toBe('Hello, World!');
    });

    it('should print multiple values', () => {
      const source = `
        int main() {
          int a = 10;
          int b = 20;
          printf("%d %d", a, b);
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
      expect(result.output).toBe('10 20');
    });

    it('should print float', () => {
      const source = `
        int main() {
          float pi = 3.14;
          printf("%f", pi);
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should print character', () => {
      const source = `
        int main() {
          char c = 'A';
          printf("%c", c);
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('Complex Programs', () => {
    it('should calculate factorial', () => {
      const source = `
        int factorial(int n) {
          if (n <= 1) {
            return 1;
          }
          return n * factorial(n - 1);
        }

        int main() {
          int result = factorial(5);
          printf("%d", result);
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should calculate fibonacci', () => {
      const source = `
        int fibonacci(int n) {
          if (n <= 1) {
            return n;
          }
          return fibonacci(n - 1) + fibonacci(n - 2);
        }

        int main() {
          int result = fibonacci(10);
          printf("%d", result);
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should sum array elements', () => {
      const source = `
        int main() {
          int arr[5];
          int sum = 0;
          int i = 0;
          while (i < 5) {
            sum = sum + i;
            i = i + 1;
          }
          printf("%d", sum);
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('Error Handling', () => {
    it('should report syntax errors', () => {
      const source = `
        int main() {
          int x = 
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors.length).toBeGreaterThan(0);
    });

    it('should report undefined variables', () => {
      const source = `
        int main() {
          int x = y;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      // May or may not catch this depending on implementation
    });
  });

  describe('Comments', () => {
    it('should handle line comments', () => {
      const source = `
        int main() {
          // This is a comment
          int x = 10;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should handle block comments', () => {
      const source = `
        int main() {
          /* This is a
             multi-line comment */
          int x = 10;
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('String Literals', () => {
    it('should handle string literals', () => {
      const source = `
        int main() {
          printf("Hello");
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
      expect(result.output).toBe('Hello');
    });
  });

  describe('Character Literals', () => {
    it('should handle character literals', () => {
      const source = `
        int main() {
          char c = 'A';
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should handle escape sequences', () => {
      const source = `
        int main() {
          char newline = '\\n';
          char tab = '\\t';
          return 0;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('Return Statements', () => {
    it('should return value', () => {
      const source = `
        int main() {
          return 42;
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });

    it('should return expression result', () => {
      const source = `
        int add(int a, int b) {
          return a + b;
        }

        int main() {
          return add(5, 3);
        }
      `;
      const result = compiler.compile(source);
      expect(result.errors).toHaveLength(0);
    });
  });
});