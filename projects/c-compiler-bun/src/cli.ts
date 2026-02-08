/**
 * CLI Entry Point - Compiles and runs .c files
 */

import { CCompiler } from "./compiler.js";
import { readFileSync } from "fs";

const args = process.argv.slice(2);

function printUsage(): void {
  console.log("Usage: cc-bun [options] <file.c>");
  console.log("");
  console.log("Options:");
  console.log("  --debug, -d       Enable debug mode");
  console.log("  --verbose, -v     Enable verbose output");
  console.log("  --ast, -a         Output AST");
  console.log("  --tokens, -t      Output tokens");
  console.log("  --help, -h        Show this help message");
  console.log("");
  console.log("Examples:");
  console.log("  cc-bun program.c");
  console.log("  cc-bun --verbose program.c");
  console.log("  cc-bun --ast program.c");
}

function parseOptions(args: string[]): { options: any, filename: string | null } {
  const options: any = {
    debug: false,
    verbose: false,
    outputAST: false,
    outputTokens: false
  };

  let filename: string | null = null;

  for (const arg of args) {
    switch (arg) {
      case "--debug":
      case "-d":
        options.debug = true;
        break;
      case "--verbose":
      case "-v":
        options.verbose = true;
        break;
      case "--ast":
      case "-a":
        options.outputAST = true;
        break;
      case "--tokens":
      case "-t":
        options.outputTokens = true;
        break;
      case "--help":
      case "-h":
        printUsage();
        process.exit(0);
        break;
      default:
        if (arg.startsWith("-")) {
          console.error(`Unknown option: ${arg}`);
          printUsage();
          process.exit(1);
        } else {
          filename = arg;
        }
        break;
    }
  }

  return { options, filename };
}

function main(): void {
  const { options, filename } = parseOptions(args);

  if (!filename) {
    console.error("Error: No input file specified");
    printUsage();
    process.exit(1);
  }

  try {
    // Read source file
    const source = readFileSync(filename, "utf-8");

    // Create compiler
    const compiler = new CCompiler(options);

    // Compile and run
    console.log(`Compiling ${filename}...`);
    const result = compiler.compileAndRun(source);

    // Output results
    if (result.success) {
      if (options.verbose) {
        console.log(`\nExecution completed in ${result.executionTime?.toFixed(2)}ms`);
      }

      if (result.errors.length > 0) {
        console.log("\nErrors:");
        for (const error of result.errors) {
          console.log(`  ${error}`);
        }
      }

      if (result.warnings.length > 0) {
        console.log("\nWarnings:");
        for (const warning of result.warnings) {
          console.log(`  ${warning}`);
        }
      }

      process.exit(0);
    } else {
      console.error("\nCompilation failed:");
      for (const error of result.errors) {
        console.error(`  ${error}`);
      }
      process.exit(1);
    }

  } catch (error) {
    console.error(`Error: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  }
}

// Run if executed directly
if (import.meta.main) {
  main();
}

export { main };