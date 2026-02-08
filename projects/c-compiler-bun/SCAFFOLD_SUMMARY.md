# Project Scaffold Summary

This document provides an overview of the C compiler project structure and architecture.

## Project Overview

A C compiler built with TypeScript and Bun using a micro-VM architecture. Each language feature has its own specialized VM module for clean separation of concerns.

## Directory Structure

```
c-compiler-bun/
├── src/                          # Source code
│   ├── core/                     # Core interfaces and types
│   │   ├── index.ts              # Core exports
│   │   ├── interfaces.ts         # Common interfaces (MicroVM, AST nodes, etc.)
│   │   └── types.ts              # Type definitions (Token, Type, etc.)
│   ├── lexer/                    # Lexer/Tokenizer
│   │   ├── index.ts              # Lexer exports
│   │   └── lexer.ts              # Tokenizer implementation
│   ├── parser/                   # Parser (Recursive Descent)
│   │   ├── index.ts              # Parser exports
│   │   └── parser.ts             # AST builder
│   ├── vm/                       # Micro-VM implementations
│   │   ├── index.ts              # VM exports
│   │   ├── base-vm.ts            # Base MicroVM class
│   │   ├── registry.ts           # MicroVM registry/dispatcher
│   │   ├── arithmetic-vm.ts      # Arithmetic operations (+, -, *, /, %)
│   │   ├── comparison-vm.ts      # Comparisons (<, >, <=, >=, ==, !=)
│   │   ├── control-flow-vm.ts    # Control flow (if/else, while, for)
│   │   ├── function-vm.ts        # Functions (def, call, return, stack)
│   │   ├── memory-vm.ts          # Memory (vars, arrays, pointers, scopes)
│   │   ├── io-vm.ts              # I/O (printf, scanf)
│   │   └── type-vm.ts            # Type checking and casting
│   ├── cli.ts                    # CLI entry point
│   ├── compiler.ts              # Main compiler orchestrator
│   └── index.ts                  # Package exports
├── test/                         # Integration tests
│   ├── examples.ts               # Example program tests
│   ├── integration.test.ts      # Integration test suite
│   └── vm.test.ts                # VM integration tests
├── tests/                        # Unit tests
│   ├── compiler.test.ts         # Compiler unit tests
│   ├── index.test.ts            # Entry point tests
│   ├── lexer.test.ts             # Lexer unit tests
│   ├── parser.test.ts            # Parser unit tests
│   ├── test-utils.ts             # Test utilities
│   └── vm.test.ts                # VM unit tests
├── examples/                     # Example C programs
│   ├── hello.c                   # Hello World
│   ├── variables.c              # Variable declarations
│   ├── arithmetic.c             # Arithmetic operations
│   ├── comparisons.c            # Comparison operations
│   ├── control-flow.c           # If/else and loops
│   ├── functions.c              # Function definitions
│   ├── arrays.c                 # Array operations
│   ├── pointers.c               # Pointer operations
│   ├── io.c                     # printf/scanf examples
│   ├── recursion.c              # Recursive functions
│   └── multi-array.c            # Multi-dimensional arrays
├── .agent/                      # Agent configuration
├── .gitignore                   # Git ignore rules
├── .eslintrc.js                 # ESLint configuration
├── .prettierrc                  # Prettier configuration
├── CHANGELOG.md                 # Changelog
├── CONTRIBUTING.md              # Contributing guidelines
├── LICENSE                      # License file
├── package.json                 # Package configuration
├── plan.json                    # Project plan
├── QUICKSTART.md                # Quick start guide
├── README.md                    # Project documentation
├── SCAFFOLD_SUMMARY.md          # This file
└── tsconfig.json                # TypeScript configuration
```

## Architecture Components

### 1. Core Interfaces (`src/core/`)

**interfaces.ts** - Defines common interfaces:
- `MicroVM` - Base interface for all VM modules
- `ASTNode` - Base interface for AST nodes
- `Expression` - Expression AST nodes
- `Statement` - Statement AST nodes
- `Program` - Complete program structure

**types.ts** - Defines type definitions:
- `TokenType` - Token type enum
- `Token` - Token structure
- `DataType` - Data type enum (int, char, float, void, pointer)
- `BinaryOperator` - Binary operator enum
- `UnaryOperator` - Unary operator enum

### 2. Lexer (`src/lexer/`)

**lexer.ts** - Tokenizer implementation:
- Scans C source code character by character
- Identifies keywords, identifiers, literals, operators
- Handles whitespace, comments, and preprocessor directives
- Produces a stream of tokens for the parser

### 3. Parser (`src/parser/`)

**parser.ts** - Recursive descent parser:
- Builds AST from token stream
- Implements grammar rules for each C construct
- Handles operator precedence and associativity
- Produces a complete AST for execution

### 4. Micro-VM Registry (`src/vm/registry.ts`)

Central dispatcher that:
- Registers all specialized VM modules
- Routes operations to appropriate VMs
- Coordinates inter-VM communication
- Maintains VM state and lifecycle

### 5. Specialized VM Modules

Each VM implements the `MicroVM` interface:

**base-vm.ts** - Base class with common VM functionality

**arithmetic-vm.ts** - Handles:
- Addition (+)
- Subtraction (-)
- Multiplication (*)
- Division (/)
- Modulo (%)

**comparison-vm.ts** - Handles:
- Less than (<)
- Greater than (>)
- Less than or equal (<=)
- Greater than or equal (>=)
- Equal (==)
- Not equal (!=)

**control-flow-vm.ts** - Handles:
- If statements
- Else statements
- Else-if chains
- While loops
- For loops
- Break and continue

**function-vm.ts** - Handles:
- Function definitions
- Function calls
- Return values
- Call stack management
- Parameter passing

**memory-vm.ts** - Handles:
- Variable declarations
- Variable assignments
- Array operations
- Pointer operations (&, *)
- Stack frames and scoping
- Memory allocation/deallocation

**io-vm.ts** - Handles:
- printf formatting and output
- scanf input parsing
- Format specifiers (%d, %c, %f, %s, %p)

**type-vm.ts** - Handles:
- Type checking
- Type casting
- Type promotion
- Type compatibility

### 6. Main Compiler (`src/compiler.ts`)

Orchestrates all components:
- Initializes lexer, parser, and VMs
- Manages compilation pipeline
- Handles errors and diagnostics
- Executes compiled code

### 7. CLI Entry Point (`src/cli.ts`)

Command-line interface:
- Parses command-line arguments
- Loads C source files
- Invokes compiler
- Displays output and errors

## Supported C Features

### Variables
- `int` - 32-bit integers
- `char` - 8-bit characters
- `float` - 32-bit floating point

### Operators
- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Comparison: `<`, `>`, `<=`, `>=`, `==`, `!=`
- Logical: `&&`, `||`, `!`
- Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`
- Assignment: `=`, `+=`, `-=`, `*=`, `/=`, `%=`
- Pointer: `&` (address-of), `*` (dereference)
- Increment/Decrement: `++`, `--`

### Control Flow
- `if`, `else`, `else if`
- `while` loops
- `for` loops
- `break`, `continue`
- `return`

### Functions
- Function declarations
- Function definitions
- Function calls
- Parameters
- Return values
- Recursion

### Data Structures
- Single-dimensional arrays
- Multi-dimensional arrays
- Pointers
- Pointer arithmetic

### I/O
- `printf` - formatted output
- `scanf` - formatted input
- Format specifiers: `%d`, `%c`, `%f`, `%s`, `%p`

### Literals
- Integer literals
- Character literals
- Float literals
- String literals

## Build and Run

### Install Dependencies
```bash
bun install
```

### Run Compiler
```bash
bun run src/cli.ts examples/hello.c
```

### Build CLI
```bash
bun run build
```

### Run Tests
```bash
bun test
```

### Type Check
```bash
bun run typecheck
```

## Development Workflow

1. **Implement Feature** - Add code to appropriate module
2. **Write Tests** - Add unit and integration tests
3. **Run Tests** - Verify all tests pass
4. **Type Check** - Ensure no type errors
5. **Test Examples** - Run example programs
6. **Update Docs** - Document changes

## Testing Strategy

### Unit Tests (`tests/`)
- Test individual components in isolation
- Mock dependencies where needed
- Cover edge cases and error conditions

### Integration Tests (`test/`)
- Test component interactions
- Test complete compilation pipeline
- Test with real C programs

### Example Tests
- Each example program has corresponding tests
- Verify expected output
- Test error handling

## Future Enhancements

- Additional data types (double, long, short)
- Struct and union support
- Enum support
- Preprocessor directives (#define, #include)
- More complex pointer operations
- File I/O (fopen, fclose, fread, fwrite)
- Standard library functions
- Optimization passes
- Code generation to native binaries

## License

MIT License - See LICENSE file for details.