/**
 * Main Compiler Module
 * Orchestrates the compilation process: lexing, parsing, and code generation
 */

const Lexer = require('./lexer');
const Parser = require('./parser');
const CodeGenerator = require('./codegen');

class Compiler {
  constructor() {
    this.lexer = null;
    this.parser = null;
    this.codegen = null;
  }

  /**
   * Compile C source code to JavaScript
   * @param {string} source - C source code
   * @returns {string} - Generated JavaScript code
   */
  compile(source) {
    // Phase 1: Lexing
    const tokens = this.tokenize(source);
    
    // Phase 2: Parsing
    const ast = this.parse(tokens);
    
    // Phase 3: Code Generation
    const jsCode = this.generate(ast);
    
    return jsCode;
  }

  /**
   * Tokenize C source code
   * @param {string} source - C source code
   * @returns {Array} - Array of tokens
   */
  tokenize(source) {
    this.lexer = new Lexer(source);
    const tokens = this.lexer.tokenize();
    return tokens;
  }

  /**
   * Parse tokens into AST
   * @param {Array} tokens - Array of tokens
   * @returns {Object} - Abstract Syntax Tree
   */
  parse(tokens) {
    this.parser = new Parser(tokens);
    const ast = this.parser.parse();
    return ast;
  }

  /**
   * Generate JavaScript from AST
   * @param {Object} ast - Abstract Syntax Tree
   * @returns {string} - Generated JavaScript code
   */
  generate(ast) {
    this.codegen = new CodeGenerator();
    const jsCode = this.codegen.generate(ast);
    return jsCode;
  }

  /**
   * Compile C source code with runtime support
   * @param {string} source - C source code
   * @returns {string} - Complete JavaScript with runtime
   */
  compileWithRuntime(source) {
    const jsCode = this.compile(source);
    
    // Add runtime support
    const runtime = `
// Runtime support for C transpilation
function print(str) {
  console.log(str);
}

// Main function entry point
if (typeof main === 'function') {
  main();
}
`;
    
    return jsCode + runtime;
  }
}

module.exports = Compiler;