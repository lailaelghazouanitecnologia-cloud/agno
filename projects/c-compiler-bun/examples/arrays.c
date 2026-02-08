/* Array Demo */
int main() {
  int arr[5];
  int i = 0;
  
  // Initialize array
  while (i < 5) {
    arr[i] = i * 10;
    i = i + 1;
  }
  
  // Print array
  i = 0;
  printf("Array elements:\n");
  while (i < 5) {
    printf("arr[%d] = %d\n", i, arr[i]);
    i = i + 1;
  }
  
  return 0;
}