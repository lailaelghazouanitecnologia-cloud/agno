/**
 * Example C programs for testing
 */

// Hello World
export const helloWorld = `
int main() {
  printf("Hello, World!\\n");
  return 0;
}
`;

// Simple arithmetic
export const arithmetic = `
int main() {
  int a = 10;
  int b = 5;
  int sum = a + b;
  int diff = a - b;
  int product = a * b;
  int quotient = a / b;
  int remainder = a % b;
  return 0;
}
`;

// Function with parameters
export const functionWithParams = `
int add(int a, int b) {
  return a + b;
}

int main() {
  int result = add(5, 3);
  return 0;
}
`;

// If-else statement
export const ifElse = `
int main() {
  int x = 10;
  if (x > 5) {
    printf("x is greater than 5\\n");
  } else {
    printf("x is not greater than 5\\n");
  }
  return 0;
}
`;

// While loop
export const whileLoop = `
int main() {
  int i = 0;
  while (i < 5) {
    i = i + 1;
  }
  return 0;
}
`;

// For loop
export const forLoop = `
int main() {
  int sum = 0;
  for (int i = 0; i < 10; i = i + 1) {
    sum = sum + i;
  }
  return 0;
}
`;

// Fibonacci
export const fibonacci = `
int fibonacci(int n) {
  if (n <= 1) {
    return n;
  }
  return fibonacci(n - 1) + fibonacci(n - 2);
}

int main() {
  int result = fibonacci(10);
  printf("Fibonacci(10) = %d\\n", result);
  return 0;
}
`;

// Factorial
export const factorial = `
int factorial(int n) {
  if (n <= 1) {
    return 1;
  }
  return n * factorial(n - 1);
}

int main() {
  int result = factorial(5);
  printf("5! = %d\\n", result);
  return 0;
}
`;

// Multiple functions
export const multipleFunctions = `
int square(int x) {
  return x * x;
}

int cube(int x) {
  return x * x * x;
}

int main() {
  int s = square(5);
  int c = cube(3);
  return 0;
}
`;

// Nested control flow
export const nestedControlFlow = `
int main() {
  int x = 5;
  int y = 10;

  if (x > 0) {
    if (y > 0) {
      printf("Both positive\\n");
    }
  }

  return 0;
}
`;

// Variable declarations
export const variableDeclarations = `
int main() {
  int x = 42;
  float pi = 3.14;
  char c = 'A';

  printf("x = %d\\n", x);
  printf("pi = %f\\n", pi);
  printf("c = %c\\n", c);

  return 0;
}
`;

// Comparison operations
export const comparisons = `
int main() {
  int a = 5;
  int b = 10;

  int lt = a < b;
  int gt = a > b;
  int le = a <= b;
  int ge = a >= b;
  int eq = a == b;
  int ne = a != b;

  return 0;
}
`;

// Complex expression
export const complexExpression = `
int main() {
  int result = (5 + 3) * 2 - 10 / 5;
  return 0;
}
`;