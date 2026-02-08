#!/usr/bin/env bun

/**
 * Scaffold Verification Script
 * 
 * This script verifies that the project scaffold is complete
 * and all necessary files and directories exist.
 */

import { existsSync } from 'fs';
import { readdir } from 'fs/promises';

interface CheckResult {
  name: string;
  exists: boolean;
  type: 'file' | 'directory';
}

const checks: CheckResult[] = [];

// Configuration files
const configFiles = [
  'package.json',
  'tsconfig.json',
  '.gitignore',
  '.eslintrc.js',
  '.prettierrc',
];

// Documentation files
const docFiles = [
  'README.md',
  'QUICKSTART.md',
  'SCAFFOLD_SUMMARY.md',
  'CHANGELOG.md',
  'CONTRIBUTING.md',
  'LICENSE',
];

// Source directories
const srcDirs = [
  'src',
  'src/core',
  'src/lexer',
  'src/parser',
  'src/vm',
];

// Source files
const srcFiles = [
  'src/index.ts',
  'src/cli.ts',
  'src/compiler.ts',
  'src/core/index.ts',
  'src/core/interfaces.ts',
  'src/core/types.ts',
  'src/lexer/index.ts',
  'src/lexer/lexer.ts',
  'src/parser/index.ts',
  'src/parser/parser.ts',
  'src/vm/index.ts',
  'src/vm/base-vm.ts',
  'src/vm/registry.ts',
  'src/vm/arithmetic-vm.ts',
  'src/vm/comparison-vm.ts',
  'src/vm/control-flow-vm.ts',
  'src/vm/function-vm.ts',
  'src/vm/memory-vm.ts',
  'src/vm/io-vm.ts',
  'src/vm/type-vm.ts',
];

// Test directories
const testDirs = [
  'test',
  'tests',
];

// Test files
const testFiles = [
  'test/examples.ts',
  'test/integration.test.ts',
  'test/vm.test.ts',
  'tests/compiler.test.ts',
  'tests/index.test.ts',
  'tests/lexer.test.ts',
  'tests/parser.test.ts',
  'tests/test-utils.ts',
  'tests/vm.test.ts',
];

// Example C files
const exampleFiles = [
  'examples/hello.c',
  'examples/variables.c',
  'examples/arithmetic.c',
  'examples/comparisons.c',
  'examples/control-flow.c',
  'examples/functions.c',
  'examples/arrays.c',
  'examples/pointers.c',
  'examples/io.c',
  'examples/recursion.c',
  'examples/multi-array.c',
];

async function checkExists(path: string, type: 'file' | 'directory'): Promise<boolean> {
  try {
    const stats = await Bun.file(path).stat();
    return type === 'file' ? stats.isFile() : stats.isDirectory();
  } catch {
    return false;
  }
}

async function runChecks() {
  console.log('🔍 Verifying Project Scaffold...\n');

  // Check configuration files
  console.log('📦 Configuration Files:');
  for (const file of configFiles) {
    const exists = await checkExists(file, 'file');
    checks.push({ name: file, exists, type: 'file' });
    console.log(`  ${exists ? '✅' : '❌'} ${file}`);
  }

  // Check documentation files
  console.log('\n📚 Documentation Files:');
  for (const file of docFiles) {
    const exists = await checkExists(file, 'file');
    checks.push({ name: file, exists, type: 'file' });
    console.log(`  ${exists ? '✅' : '❌'} ${file}`);
  }

  // Check source directories
  console.log('\n📁 Source Directories:');
  for (const dir of srcDirs) {
    const exists = await checkExists(dir, 'directory');
    checks.push({ name: dir, exists, type: 'directory' });
    console.log(`  ${exists ? '✅' : '❌'} ${dir}/`);
  }

  // Check source files
  console.log('\n📄 Source Files:');
  for (const file of srcFiles) {
    const exists = await checkExists(file, 'file');
    checks.push({ name: file, exists, type: 'file' });
    console.log(`  ${exists ? '✅' : '❌'} ${file}`);
  }

  // Check test directories
  console.log('\n🧪 Test Directories:');
  for (const dir of testDirs) {
    const exists = await checkExists(dir, 'directory');
    checks.push({ name: dir, exists, type: 'directory' });
    console.log(`  ${exists ? '✅' : '❌'} ${dir}/`);
  }

  // Check test files
  console.log('\n📝 Test Files:');
  for (const file of testFiles) {
    const exists = await checkExists(file, 'file');
    checks.push({ name: file, exists, type: 'file' });
    console.log(`  ${exists ? '✅' : '❌'} ${file}`);
  }

  // Check example files
  console.log('\n💡 Example C Programs:');
  for (const file of exampleFiles) {
    const exists = await checkExists(file, 'file');
    checks.push({ name: file, exists, type: 'file' });
    console.log(`  ${exists ? '✅' : '❌'} ${file}`);
  }

  // Summary
  const total = checks.length;
  const passed = checks.filter(c => c.exists).length;
  const failed = total - passed;

  console.log('\n' + '='.repeat(50));
  console.log(`📊 Summary: ${passed}/${total} checks passed`);
  if (failed > 0) {
    console.log(`⚠️  ${failed} checks failed`);
  } else {
    console.log('✅ All checks passed! Scaffold is complete.');
  }
  console.log('='.repeat(50));

  process.exit(failed > 0 ? 1 : 0);
}

runChecks().catch(console.error);