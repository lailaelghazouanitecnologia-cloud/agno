/**
 * Test Runner for C Compiler
 * Runs all test files and reports results
 */

const fs = require('fs');
const path = require('path');
const Compiler = require('../src/compiler');

class TestRunner {
  constructor() {
    this.passed = 0;
    this.failed = 0;
    this.errors = [];
  }

  run() {
    console.log('Running C Compiler Tests...\n');
    
    // Get all test files
    const testDir = path.join(__dirname);
    const files = fs.readdirSync(testDir)
      .filter(f => f.endsWith('.test.js'));
    
    for (const file of files) {
      this.runTestFile(path.join(testDir, file));
    }
    
    // Print summary
    console.log('\n' + '='.repeat(50));
    console.log(`Tests passed: ${this.passed}`);
    console.log(`Tests failed: ${this.failed}`);
    console.log('='.repeat(50));
    
    if (this.errors.length > 0) {
      console.log('\nFailed tests:');
      for (const error of this.errors) {
        console.log(`  - ${error}`);
      }
    }
    
    return this.failed === 0 ? 0 : 1;
  }

  runTestFile(filePath) {
    const testName = path.basename(filePath, '.test.js');
    console.log(`\n${testName}:`);
    
    try {
      const testModule = require(filePath);
      
      if (typeof testModule.run === 'function') {
        testModule.run(this);
      } else {
        console.log('  Skipped (no run function)');
      }
    } catch (error) {
      this.failed++;
      this.errors.push(`${testName}: ${error.message}`);
      console.log(`  Error: ${error.message}`);
    }
  }

  assert(condition, message) {
    if (condition) {
      this.passed++;
      console.log(`  ✓ ${message}`);
    } else {
      this.failed++;
      this.errors.push(message);
      console.log(`  ✗ ${message}`);
    }
  }

  assertEqual(actual, expected, message) {
    if (actual === expected) {
      this.passed++;
      console.log(`  ✓ ${message}`);
    } else {
      this.failed++;
      this.errors.push(`${message} (expected: ${expected}, got: ${actual})`);
      console.log(`  ✗ ${message}`);
      console.log(`    Expected: ${expected}`);
      console.log(`    Got: ${actual}`);
    }
  }

  test(name, fn) {
    try {
      fn();
    } catch (error) {
      this.failed++;
      this.errors.push(`${name}: ${error.message}`);
      console.log(`  ✗ ${name}: ${error.message}`);
    }
  }

  assertContains(haystack, needle, message) {
    if (haystack.includes(needle)) {
      this.passed++;
      console.log(`  ✓ ${message}`);
    } else {
      this.failed++;
      this.errors.push(`${message} (string not found)`);
      console.log(`  ✗ ${message}`);
      console.log(`    Expected to contain: ${needle}`);
    }
  }
}

// Run tests if executed directly
if (require.main === module) {
  const runner = new TestRunner();
  process.exit(runner.run());
}

module.exports = TestRunner;