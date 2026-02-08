/* Fibonacci Sequence */
int fibonacci(int n) {
  if (n <= 1) {
    return n;
  }
  return fibonacci(n - 1) + fibonacci(n - 2);
}

int main() {
  int n = 10;
  printf("Fibonacci sequence:\n");
  
  for (int i = 0; i < n; i = i + 1) {
    int result = fibonacci(i);
    printf("%d ", result);
  }
  
  printf("\n");
  return 0;
}