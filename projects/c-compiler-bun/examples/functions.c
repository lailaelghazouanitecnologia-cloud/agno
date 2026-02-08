// Functions example
int add(int a, int b) {
  return a + b;
}

int multiply(int a, int b) {
  return a * b;
}

int factorial(int n) {
  if (n <= 1) {
    return 1;
  }
  return n * factorial(n - 1);
}

int main() {
  int sum = add(10, 20);
  int product = multiply(5, 6);
  int fact = factorial(5);
  
  printf("Sum: %d\n", sum);
  printf("Product: %d\n", product);
  printf("Factorial of 5: %d\n", fact);
  
  return 0;
}