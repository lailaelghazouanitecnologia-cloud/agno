# Quick Start Guide

Get up and running with the C compiler in minutes.

## Prerequisites

- [Bun](https://bun.sh/) v1.0.0 or later
- Git (optional, for cloning)

## Installation

1. **Clone or navigate to the project directory:**
   ```bash
   cd /path/to/c-compiler-bun
   ```

2. **Install dependencies:**
   ```bash
   bun install
   ```

## Your First Program

Let's compile and run a simple "Hello, World!" program.

1. **Create a C file** (or use the provided example):
   ```c
   // hello.c
   int main() {
       printf("Hello, World!\n");
       return 0;
   }
   ```

2. **Run the compiler:**
   ```bash
   bun run src/cli.ts hello.c
   ```

3. **Expected output:**
   ```
   Hello, World!
   ```

## Running Example Programs

The project includes several example programs demonstrating different features:

```bash
# Hello World
bun run src/cli.ts examples/hello.c

# Variables
bun run src/cli.ts examples/variables.c

# Arithmetic
bun run src/cli.ts examples/arithmetic.c

# Comparisons
bun run src/cli.ts examples/comparisons.c

# Control Flow
bun run src/cli.ts examples/control-flow.c

# Functions
bun run src/cli.ts examples/functions.c

# Arrays
bun run src/cli.ts examples/arrays.c

# Pointers
bun run src/cli.ts examples/pointers.c

# I/O
bun run src/cli.ts examples/io.c

# Recursion
bun run src/cli.ts examples/recursion.c

# Multi-dimensional Arrays
bun run src/cli.ts examples/multi-array.c
```

## Building the CLI

To build a standalone executable:

```bash
bun run build
```

This creates a `dist/` directory with the compiled CLI.

## Running Tests

Run the complete test suite:

```bash
bun test
```

Run tests in watch mode (for development):

```bash
bun test --watch
```

Run tests with coverage:

```bash
bun test --coverage
```

## Development Commands

### Type Checking
Check for TypeScript errors:
```bash
bun run typecheck
```

### Linting
Run ESLint:
```bash
bun run lint
```

### Formatting
Format code with Prettier:
```bash
bun run format
```

## Project Structure Overview

```
src/
├── core/          # Core interfaces and types
├── lexer/         # Tokenizer
├── parser/        # AST builder
├── vm/            # Micro-VM implementations
├── cli.ts         # CLI entry point
├── compiler.ts    # Main compiler
└── index.ts       # Package exports

test/              # Integration tests
tests/             # Unit tests
examples/          # Example C programs
```

## Writing Your Own C Programs

Create a new `.c` file and use the compiler:

```bash
# Create myprogram.c
cat > myprogram.c << 'EOF'
int main() {
    int x = 42;
    printf("The answer is: %d\n", x);
    return 0;
}
EOF

# Run it
bun run src/cli.ts myprogram.c
```

## Supported Features

### Variables
```c
int x = 10;
char c = 'A';
float f = 3.14;
```

### Arithmetic
```c
int result = a + b * c - d / e;
```

### Comparisons
```c
if (x > 5 && y < 10) {
    // ...
}
```

### Control Flow
```c
if (condition) {
    // ...
} else if (other_condition) {
    // ...
} else {
    // ...
}

while (condition) {
    // ...
}

for (int i = 0; i < 10; i++) {
    // ...
}
```

### Functions
```c
int add(int a, int b) {
    return a + b;
}

int main() {
    int sum = add(5, 3);
    return 0;
}
```

### Arrays
```c
int arr[5] = {1, 2, 3, 4, 5};
printf("%d\n", arr[0]);
```

### Pointers
```c
int x = 10;
int* ptr = &x;
printf("%d\n", *ptr);
```

### I/O
```c
printf("Enter a number: ");
scanf("%d", &x);
printf("You entered: %d\n", x);
```

## Troubleshooting

### "command not found: bun"
Install Bun from https://bun.sh/

### Type errors
Run `bun run typecheck` to see detailed error messages

### Tests failing
Run `bun test` to see which tests are failing and why

### Unexpected behavior
Check the example programs to see expected behavior

## Next Steps

1. Read the [README.md](README.md) for detailed documentation
2. Explore the [SCAFFOLD_SUMMARY.md](SCAFFOLD_SUMMARY.md) for architecture details
3. Check out the source code in `src/`
4. Run the tests to understand the system
5. Try modifying example programs

## Getting Help

- Check the documentation in `README.md`
- Review the architecture in `SCAFFOLD_SUMMARY.md`
- Look at example programs in `examples/`
- Examine test files in `tests/` and `test/`

Happy compiling! 🚀