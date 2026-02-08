int main() {
    int x;
    char c;
    float f;
    
    // printf examples
    printf("Enter an integer: ");
    scanf("%d", &x);
    printf("You entered: %d\n", x);
    
    printf("Enter a character: ");
    scanf(" %c", &c);
    printf("You entered: %c\n", c);
    
    printf("Enter a float: ");
    scanf("%f", &f);
    printf("You entered: %f\n", f);
    
    // String literal
    printf("Hello from C compiler!\n");
    
    return 0;
}