#!/usr/bin/env node

/**
 * CLI for TypeScript transpiler
 */

import * as fs from "fs";
import * as path from "path";
import { transpile } from "./index";

interface CLIOptions {
  input?: string;
  output?: string;
  watch?: boolean;
  help?: boolean;
  version?: boolean;
}

function parseArgs(args: string[]): CLIOptions {
  const options: CLIOptions = {};

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];

    switch (arg) {
      case "-o":
      case "--output":
        options.output = args[++i];
        break;
      case "-w":
      case "--watch":
        options.watch = true;
        break;
      case "-h":
      case "--help":
        options.help = true;
        break;
      case "-v":
      case "--version":
        options.version = true;
        break;
      default:
        if (!arg.startsWith("-")) {
          options.input = arg;
        }
        break;
    }
  }

  return options;
}

function showHelp(): void {
  console.log(`
TypeScript to JavaScript Transpiler

Usage:
  ts-transpile [options] <input-file>

Options:
  -o, --output <file>    Output file path
  -w, --watch           Watch mode (recompile on changes)
  -h, --help            Show this help message
  -v, --version         Show version number

Examples:
  ts-transpile input.ts
  ts-transpile input.ts -o output.js
  ts-transpile input.ts -o output.js --watch
`);
}

function showVersion(): void {
  console.log("ts-transpiler v1.0.0");
}

function transpileFile(inputPath: string, outputPath?: string): void {
  try {
    // Read input file
    const source = fs.readFileSync(inputPath, "utf-8");

    // Transpile
    const jsCode = transpile(source);

    // Determine output path
    if (!outputPath) {
      const parsed = path.parse(inputPath);
      outputPath = path.join(parsed.dir, parsed.name + ".js");
    }

    // Write output file
    fs.writeFileSync(outputPath, jsCode, "utf-8");

    console.log(`✓ Transpiled ${inputPath} -> ${outputPath}`);
  } catch (error) {
    console.error(`Error transpiling ${inputPath}:`);
    console.error(error instanceof Error ? error.message : error);
    process.exit(1);
  }
}

function watchFile(inputPath: string, outputPath?: string): void {
  console.log(`Watching ${inputPath} for changes...`);

  let lastModified = fs.statSync(inputPath).mtimeMs;

  const check = () => {
    try {
      const stats = fs.statSync(inputPath);
      if (stats.mtimeMs > lastModified) {
        lastModified = stats.mtimeMs;
        transpileFile(inputPath, outputPath);
      }
    } catch (error) {
      // Ignore errors during watch
    }
  };

  setInterval(check, 1000);
}

function main(): void {
  const args = process.argv.slice(2);
  const options = parseArgs(args);

  if (options.help) {
    showHelp();
    return;
  }

  if (options.version) {
    showVersion();
    return;
  }

  if (!options.input) {
    console.error("Error: No input file specified");
    console.error("Use --help for usage information");
    process.exit(1);
  }

  if (options.watch) {
    watchFile(options.input, options.output);
  } else {
    transpileFile(options.input, options.output);
  }
}

main();