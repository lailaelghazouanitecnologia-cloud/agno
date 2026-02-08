#!/usr/bin/env bun
/**
 * Verify scaffold - checks that all required files exist
 */

import { existsSync } from 'fs';
import { join } from 'path';

const requiredFiles = [
  // Root
  'package.json',
  'tsconfig.json',
  'plan.json',
  
  // Core
  'src/core/interfaces.ts',
  'src/core/types.ts',
  'src/core/index.ts',
  
  // Lexer
  'src/lexer/lexer.ts',
  'src/lexer/index.ts',
  
  // Parser
  'src/parser/parser.ts',
  'src/parser/index.ts',
  
  // VM
  'src/vm/MicroVM.ts',
  'src/vm/base-vm.ts',
  'src/vm/registry.ts',
  'src/vm/arithmetic-vm.ts',
  'src/vm/comparison-vm.ts',
  'src/vm/control-flow-vm.ts',
  'src/vm/function-vm.ts',
  'src/vm/memory-vm.ts',
  'src/vm/io-vm.ts',
  'src/vm/type-vm.ts',
  'src/vm/index.ts',
  
  // VMS
  'src/vms/arithmetic-vm.ts',
  'src/vms/comparison-vm.ts',
  'src/vms/control-flow-vm.ts',
  'src/vms/function-vm.ts',
  'src/vms/io-vm.ts',
  'src/vms/memory-vm.ts',
  'src/vms/type-vm.ts',
  
  // Main
  'src/interfaces.ts',
  'src/types.ts',
  'src/lexer.ts',
  'src/parser.ts',
  'src/microvm-registry.ts',
  'src/arithmetic-vm.ts',
  'src/comparison-vm.ts',
  'src/control-flow-vm.ts',
  'src/function-vm.ts',
  'src/memory-vm.ts',
  'src/io-vm.ts',
  'src/type-vm.ts',
  'src/compiler.ts',
  'src/cli.ts',
  'src/index.ts',
  
  // Tests
  'tests/lexer.test.ts',
  'tests/parser.test.ts',
  'tests/vm.test.ts',
  'tests/compiler.test.ts',
  'tests/integration.test.ts',
  'tests/test-utils.ts',
  'tests/index.test.ts',
  
  'test/vm.test.ts',
  'test/integration.test.ts',
  'test/examples.ts',
  
  // Examples
  'examples/hello.c',
  'examples/variables.c',
  'examples/arithmetic.c',
  'examples/comparisons.c',
  'examples/ifelse.c',
  'examples/while.c',
  'examples/for.c',
  'examples/loops.c',
  'examples/function.c',
  'examples/functions.c',
  'examples/factorial.c',
  'examples/fibonacci.c',
  'examples/recursion.c',
  'examples/arrays.c',
  'examples/multi-array.c',
  'examples/pointers.c',
  'examples/io.c',
  'examples/control-flow.c',
];

const root = process.cwd();
let missing = 0;
let existing = 0;

console.log('🔍 Verifying scaffold...\n');

for (const file of requiredFiles) {
  const fullPath = join(root, file);
  if (existsSync(fullPath)) {
    console.log(`✅ ${file}`);
    existing++;
  } else {
    console.log(`❌ ${file} - MISSING`);
    missing++;
  }
}

console.log(`\n📊 Summary: ${existing} existing, ${missing} missing\n`);

if (missing === 0) {
  console.log('✨ Scaffold is complete!');
  process.exit(0);
} else {
  console.log('⚠️  Scaffold is incomplete!');
  process.exit(1);
}