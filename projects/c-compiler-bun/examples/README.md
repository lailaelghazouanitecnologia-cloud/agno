# Examples

This directory contains example C programs that can be compiled and run with the C compiler.

## Running Examples

Using the CLI:

```bash
bun run cli examples/hello.c
bun run cli examples/fibonacci.c
bun run cli examples/functions.c
```

Using the programmatic API:

```typescript
import { Compiler } from './src/index.js';
import { readFileSync } from 'fs';

const compiler = new Compiler();
const source = readFileSync('examples/hello.c', 'utf-8');
const result = compiler.compileAndRun(source);

console.log(`Exit code: ${result.exitCode}`);
console.log(`Output: ${result.output}`);
```

## Example Files

- `hello.c` - Simple program that returns 42
- `arithmetic.c` - Demonstrates arithmetic operations
- `control-flow.c` - Demonstrates if/else, while, and for loops
- `functions.c` - Demonstrates function definitions and calls
- `fibonacci.c` - Recursive Fibonacci calculation
- `variables.c` - Demonstrates variable declarations and types
- `arrays.c` - Demonstrates array usage
- `pointers.c` - Demonstrates pointer operations