// Factorial function
int factorial(int n);

int main() {
    int n = 5;
    int result = factorial(n);
    printf("factorial(%d) = %d\n", n, result);
    return 0;
}

int factorial(int n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}