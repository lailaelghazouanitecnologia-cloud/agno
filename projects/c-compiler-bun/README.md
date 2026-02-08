# C Compiler Bun

A C compiler built with TypeScript and Bun using a micro-VM architecture.

## Features

- **Variables**: int, char, float
- **Arithmetic**: +, -, *, /, %
- **Comparisons**: <, >, <=, >=, ==, !=
- **Control Flow**: if/else/else-if, while, for loops
- **Functions**: parameters, return values, call stack
- **Arrays**: multi-dimensional support
- **Pointers**: basic & and * operators
- **I/O**: printf/scanf
- **String Literals**: full support

## Architecture

The compiler uses a micro-VM architecture where each language feature has its own specialized VM module:

1. **Lexer** - Tokenizes C source code
2. **Parser** - Recursive descent parser building AST
3. **MicroVM Registry** - Dispatches operations to specialized VMs
4. **ArithmeticVM** - Handles +, -, *, /, % operations
5. **ComparisonVM** - Handles relational and equality ops
6. **ControlFlowVM** - Handles if/else, while, for
7. **FunctionVM** - Handles function definitions, calls, returns
8. **MemoryVM** - Handles variables, arrays, pointers, scoping
9. **IOVM** - Handles printf/scanf
10. **TypeVM** - Handles type checking and casting

## Installation

```bash
bun install
```

## Usage

### Compile and Run a C File

```bash
bun run src/cli.ts examples/hello.c
```

### Build the CLI

```bash
bun run build
```

### Run Tests

```bash
bun test
```

### Watch Tests

```bash
bun test --watch
```

## Project Structure

```
c-compiler-bun/
├── src/
│   ├── core/           # Core interfaces and types
│   ├── lexer/          # Tokenizer
│   ├── parser/         # AST builder
│   ├── vm/             # Micro-VM implementations
│   ├── cli.ts          # CLI entry point
│   ├── compiler.ts     # Main compiler orchestrator
│   └── index.ts        # Package exports
├── test/               # Integration tests
├── tests/              # Unit tests
├── examples/           # Example C programs
└── package.json
```

## Example C Programs

See the `examples/` directory for sample C programs:

- `hello.c` - Hello World
- `variables.c` - Variable declarations
- `arithmetic.c` - Arithmetic operations
- `control-flow.c` - If/else and loops
- `functions.c` - Function definitions and calls
- `arrays.c` - Array operations
- `pointers.c` - Pointer operations
- `io.c` - printf/scanf examples

## Development

### Type Checking

```bash
bun run typecheck
```

### Linting

```bash
bun run lint
```

### Formatting

```bash
bun run format
```

## License

MIT