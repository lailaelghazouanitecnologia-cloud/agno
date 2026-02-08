/**
 * Main transpiler entry point
 */

import { Parser } from "./parser";
import { CodeGenerator } from "./codegen";

export interface TranspileOptions {
  sourceMap?: boolean;
  removeComments?: boolean;
  target?: "es5" | "es6" | "es2020";
}

export class Transpiler {
  private options: TranspileOptions;

  constructor(options: TranspileOptions = {}) {
    this.options = {
      sourceMap: false,
      removeComments: false,
      target: "es2020",
      ...options,
    };
  }

  public transpile(source: string): string {
    // Parse TypeScript into AST
    const parser = new Parser(source);
    const ast = parser.parse();

    // Generate JavaScript from AST
    const codeGenerator = new CodeGenerator();
    const jsCode = codeGenerator.generate(ast);

    return jsCode;
  }

  public transpileFile(tsFilePath: string): string {
    const fs = require("fs");
    const source = fs.readFileSync(tsFilePath, "utf-8");
    return this.transpile(source);
  }
}

/**
 * Convenience function to transpile TypeScript to JavaScript
 */
export function transpile(source: string, options?: TranspileOptions): string {
  const transpiler = new Transpiler(options);
  return transpiler.transpile(source);
}

export default Transpiler;