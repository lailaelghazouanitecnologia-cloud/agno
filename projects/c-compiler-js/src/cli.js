#!/usr/bin/env node

/**
 * CLI Entry Point for C Compiler
 * Reads a .c file and outputs .js
 */

const fs = require('fs');
const path = require('path');
const Compiler = require('./compiler');

function printUsage() {
  console.log('Usage: c-compiler-js <input.c> [output.js]');
  console.log('');
  console.log('Options:');
  console.log('  -h, --help     Show this help message');
  console.log('  -v, --version  Show version information');
  console.log('  -r, --runtime  Include runtime support (print function, main entry)');
  console.log('');
  console.log('Examples:');
  console.log('  c-compiler-js program.c');
  console.log('  c-compiler-js program.c program.js');
  console.log('  c-compiler-js program.c -r  # with runtime');
}

function printVersion() {
  const packageJson = require('../package.json');
  console.log(`C Compiler JS v${packageJson.version}`);
}

function main() {
  const args = process.argv.slice(2);
  
  if (args.length === 0) {
    printUsage();
    process.exit(1);
  }
  
  // Parse arguments
  let inputFile = null;
  let outputFile = null;
  let includeRuntime = false;
  
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    
    if (arg === '-h' || arg === '--help') {
      printUsage();
      process.exit(0);
    } else if (arg === '-v' || arg === '--version') {
      printVersion();
      process.exit(0);
    } else if (arg === '-r' || arg === '--runtime') {
      includeRuntime = true;
    } else if (!inputFile) {
      inputFile = arg;
    } else if (!outputFile) {
      outputFile = arg;
    } else {
      console.error(`Error: Unexpected argument: ${arg}`);
      printUsage();
      process.exit(1);
    }
  }
  
  if (!inputFile) {
    console.error('Error: No input file specified');
    printUsage();
    process.exit(1);
  }
  
  // Check if input file exists
  if (!fs.existsSync(inputFile)) {
    console.error(`Error: Input file not found: ${inputFile}`);
    process.exit(1);
  }
  
  // Determine output file
  if (!outputFile) {
    const parsed = path.parse(inputFile);
    outputFile = path.join(parsed.dir, parsed.name + '.js');
  }
  
  try {
    // Read input file
    console.log(`Reading: ${inputFile}`);
    const source = fs.readFileSync(inputFile, 'utf-8');
    
    // Compile
    console.log('Compiling...');
    const compiler = new Compiler();
    let jsCode;
    
    if (includeRuntime) {
      jsCode = compiler.compileWithRuntime(source);
    } else {
      jsCode = compiler.compile(source);
    }
    
    // Write output file
    console.log(`Writing: ${outputFile}`);
    fs.writeFileSync(outputFile, jsCode, 'utf-8');
    
    console.log('Compilation successful!');
    console.log(`Output: ${outputFile}`);
    
  } catch (error) {
    console.error(`Compilation error: ${error.message}`);
    process.exit(1);
  }
}

// Run if executed directly
if (require.main === module) {
  main();
}

module.exports = main;