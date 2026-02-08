# TypeScript to JavaScript Transpiler

A TypeScript to JavaScript transpiler built from scratch in TypeScript.

## Features

- **Type Erasure**: Removes all TypeScript type annotations
- **Interface Removal**: Strips interface declarations
- **Type Alias Removal**: Removes type alias declarations
- **Enum Transpilation**: Converts enums to JavaScript objects
- **Class Support**: Transpiles classes with methods and properties
- **Function Support**: Handles regular functions, arrow functions, and async functions
- **Import/Export**: Preserves ES module import/export syntax
- **Control Flow**: Supports if/else, for loops, while loops
- **Access Modifiers**: Removes public/private/protected modifiers

## Installation

```bash
npm install
```

## Building

```bash
npm run build
```

## Usage

### CLI

```bash
# Transpile a file
node dist/cli.js input.ts

# Specify output file
node dist/cli.js input.ts -o output.js

# Watch mode
node dist/cli.js input.ts -o output.js --watch
```

### Programmatic API

```typescript
import { transpile } from "./dist/index";

const tsCode = `
  function add(a: number, b: number): number {
    return a + b;
  }
`;

const jsCode = transpile(tsCode);
console.log(jsCode);
```

## Running Tests

```bash
npm test
```

## Examples

### Input (TypeScript)

```typescript
interface User {
  name: string;
  age: number;
}

class Person {
  private name: string;
  public age: number;

  constructor(name: string, age: number) {
    this.name = name;
    this.age = age;
  }

  greet(): string {
    return \`Hello, I'm \${this.name}\`;
  }
}

function add(a: number, b: number): number {
  return a + b;
}

const multiply = (a: number, b: number): number => a * b;
```

### Output (JavaScript)

```javascript
class Person {
  name;
  age;

  constructor(name, age) {
    this.name = name;
    this.age = age;
  }

  greet() {
    return "Hello, I'm " + this.name;
  }
}

function add(a, b) {
  return a + b;
}

const multiply = (a, b) => a * b;
```

## Architecture

The transpiler consists of four main components:

1. **Lexer** (`src/lexer.ts`): Tokenizes TypeScript source code
2. **Parser** (`src/parser.ts`): Builds an Abstract Syntax Tree (AST) from tokens
3. **Code Generator** (`src/codegen.ts`): Generates JavaScript code from the AST
4. **Transpiler** (`src/index.ts`): Main entry point that orchestrates the process

## Supported TypeScript Features

- ✓ Variable declarations (let, const, var)
- ✓ Type annotations (removed)
- ✓ Function declarations and expressions
- ✓ Arrow functions
- ✓ Class declarations
- ✓ Interface declarations (removed)
- ✓ Type aliases (removed)
- ✓ Enum declarations
- ✓ Import/Export statements
- ✓ Async/await
- ✓ Generics (type parameters removed)
- ✓ Optional parameters
- ✓ Rest parameters
- ✓ Access modifiers (removed)
- ✓ Control flow statements
- ✓ Array and object literals

## Limitations

- Does not perform type checking
- Limited error recovery
- Does not support all TypeScript features (e.g., decorators, namespaces)
- No source map generation
- No module bundling

## License

MIT