/**
 * Test utilities shared across test files
 */

import { Compiler } from '../src/index.js';

export function createCompiler(): Compiler {
  return new Compiler();
}

export function expectSuccess(result: any): void {
  if (!result.success) {
    throw new Error(`Expected success but got errors: ${result.errors?.join(', ')}`);
  }
}

export function expectExitCode(result: any, expected: number): void {
  expectSuccess(result);
  if (result.exitCode !== expected) {
    throw new Error(`Expected exit code ${expected} but got ${result.exitCode}`);
  }
}