/**
 * Integration tests using example programs
 */

import { describe, it, expect, beforeAll, afterAll } from 'bun:test';
import { Compiler } from '../src/index.js';
import * as examples from './examples.js';

describe('Integration Tests', () => {
  let compiler: Compiler;

  beforeAll(() => {
    compiler = new Compiler({ debug: false });
  });

  afterAll(() => {
    compiler.reset();
  });

  describe('Hello World', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.helloWorld);

      expect(result.success).toBe(true);
      expect(result.exitCode).toBe(0);
    });
  });

  describe('Arithmetic', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.arithmetic);

      expect(result.success).toBe(true);
    });
  });

  describe('Function with Parameters', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.functionWithParams);

      expect(result.success).toBe(true);
    });
  });

  describe('If-Else', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.ifElse);

      expect(result.success).toBe(true);
    });
  });

  describe('While Loop', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.whileLoop);

      expect(result.success).toBe(true);
    });
  });

  describe('For Loop', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.forLoop);

      expect(result.success).toBe(true);
    });
  });

  describe('Fibonacci', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.fibonacci);

      expect(result.success).toBe(true);
    });
  });

  describe('Factorial', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.factorial);

      expect(result.success).toBe(true);
    });
  });

  describe('Multiple Functions', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.multipleFunctions);

      expect(result.success).toBe(true);
    });
  });

  describe('Nested Control Flow', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.nestedControlFlow);

      expect(result.success).toBe(true);
    });
  });

  describe('Variable Declarations', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.variableDeclarations);

      expect(result.success).toBe(true);
    });
  });

  describe('Comparisons', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.comparisons);

      expect(result.success).toBe(true);
    });
  });

  describe('Complex Expression', () => {
    it('should compile and run', async () => {
      const result = await compiler.compileAndRun(examples.complexExpression);

      expect(result.success).toBe(true);
    });
  });

  describe('Error Handling', () => {
    it('should handle syntax errors', async () => {
      const source = 'int main { missing parenthesis';

      const result = await compiler.compileAndRun(source);

      expect(result.success).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
    });

    it('should handle undefined functions', async () => {
      const source = `
        int main() {
          foo();
          return 0;
        }
      `;

      const result = await compiler.compileAndRun(source);

      expect(result.success).toBe(false);
    });
  });

  describe('Compiler Options', () => {
    it('should respect verbose option', async () => {
      const testCompiler = new Compiler({ verbose: true });
      const result = await testCompiler.compileAndRun(examples.helloWorld);

      expect(result.success).toBe(true);
    });

    it('should respect debug option', async () => {
      const testCompiler = new Compiler({ debug: true });
      const result = await testCompiler.compileAndRun(examples.helloWorld);

      expect(result.success).toBe(true);
    });
  });

  describe('State Management', () => {
    it('should reset compiler state', async () => {
      await compiler.compileAndRun(examples.helloWorld);
      compiler.reset();

      const result = await compiler.compileAndRun(examples.arithmetic);

      expect(result.success).toBe(true);
    });

    it('should handle multiple compilations', async () => {
      const result1 = await compiler.compileAndRun(examples.helloWorld);
      const result2 = await compiler.compileAndRun(examples.arithmetic);
      const result3 = await compiler.compileAndRun(examples.functionWithParams);

      expect(result1.success).toBe(true);
      expect(result2.success).toBe(true);
      expect(result3.success).toBe(true);
    });
  });
});