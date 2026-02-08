/* Pointer Demo */
int main() {
  int x = 42;
  int* ptr = &x;
  int y = *ptr;
  
  printf("x = %d\n", x);
  printf("Address of x = %d\n", ptr);
  printf("Value at ptr = %d\n", y);
  
  return 0;
}