/*
 * Programa Fibonacci en C
 * Demuestra: recursión, funciones, condicionales, variables
 */

int fibonacci(int n) {
    if (n <= 1) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

int main() {
    int n;
    int i;
    int result;
    
    n = 10;
    
    printf("Calculando fibonacci(%d):\n", n);
    
    for (i = 0; i <= n; i++) {
        result = fibonacci(i);
        printf("  fibonacci(%d) = %d\n", i, result);
    }
    
    return 0;
}