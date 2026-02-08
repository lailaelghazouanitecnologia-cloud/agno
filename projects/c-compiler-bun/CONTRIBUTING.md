# Contributing to C Compiler in TypeScript with Bun

## Development Setup

1. Clone the repository
2. Run `bun install`
3. Run `bun test` to verify everything works

## Code Style

- Use TypeScript strict mode
- Follow existing naming conventions
- Add JSDoc comments for public APIs
- Write tests for new features

## Adding Features

### Adding a New Micro-VM

1. Create the VM class in `src/vm/`
2. Implement the `IMicroVM` interface
3. Register it in `src/compiler.ts`
4. Add tests in `test/vm.test.ts`

### Adding Language Support

1. Update `src/core/types.ts` with new token/node types
2. Extend the Lexer in `src/lexer/lexer.ts`
3. Extend the Parser in `src/parser/parser.ts`
4. Update appropriate VMs
5. Add integration tests

## Testing

Run all tests:

```bash
bun test
```

Run specific test file:

```bash
bun test test/lexer.test.ts
```

Run with watch mode:

```bash
bun test --watch
```

## Project Goals

- [ ] Complete Lexer implementation
- [ ] Complete Parser implementation
- [ ] Complete all Micro-VM implementations
- [ ] Full integration testing
- [ ] Performance optimization
- [ ] Error reporting improvements
- [ ] More example programs