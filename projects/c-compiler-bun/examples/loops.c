/* Loop Demo */
int main() {
  int sum = 0;
  
  // For loop
  for (int i = 0; i < 10; i = i + 1) {
    sum = sum + i;
  }
  
  printf("Sum of 0-9: %d\n", sum);
  
  // While loop
  int count = 0;
  while (count < 5) {
    printf("Count: %d\n", count);
    count = count + 1;
  }
  
  return 0;
}