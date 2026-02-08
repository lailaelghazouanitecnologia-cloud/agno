/**
 * Compiler - Main orchestrator for the C compiler
 * 
 * This class coordinates the lexer, parser, and all VMs to compile and run C code.
 */

import { Lexer } from './lexer.js';
import { Parser } from './parser.js';
import { VMRegistry } from './microvm-registry.js';
import { ArithmeticVM } from './arithmetic-vm.js';
import { ComparisonVM } from './comparison-vm.js';
import { ControlFlowVM } from './control-flow-vm.js';
import { FunctionVM } from './function-vm.js';
import { MemoryVM } from './memory-vm.js';
import { IOVM } from './io-vm.js';
import { TypeVM } from './type-vm.js';
import {
  ASTNode,
  ExecutionContext,
  ExecutionResult,
  CompilationResult,
  CompilationError,
  CompilationWarning,
  MemoryState,
  CallStack,
} from './interfaces.js';
import { NodeType } from './types.js';

/**
 * Compiler - Main compiler orchestrator
 */
export class Compiler {
  private lexer: Lexer;
  private parser: Parser;
  private vmRegistry: VMRegistry;
  private memoryVM: MemoryVM;
  private functionVM: FunctionVM;
  private ioVM: IOVM;

  constructor() {
    this.lexer = new Lexer();
    this.parser = new Parser();
    this.vmRegistry = new VMRegistry();
    this.memoryVM = new MemoryVM();
    this.functionVM = new FunctionVM();
    this.ioVM = new IOVM();

    this.initializeVMs();
  }

  /**
   * Compile source code
   */
  compile(source: string): CompilationResult {
    const errors: CompilationError[] = [];
    const warnings: CompilationWarning[] = [];

    try {
      // Tokenize
      const tokens = this.lexer.tokenize(source);

      // Parse
      const ast = this.parser.parse(tokens);

      // Type checking (placeholder)
      // This would be expanded to do full type checking

      return {
        success: true,
        ast,
        errors,
        warnings,
      };
    } catch (error) {
      if (error instanceof Error) {
        errors.push({
          message: error.message,
          line: 0,
          column: 0,
          type: 'syntax',
        });
      }

      return {
        success: false,
        errors,
        warnings,
      };
    }
  }

  /**
   * Run compiled code
   */
  run(ast: ASTNode, input: string[] = []): ExecutionResult {
    try {
      // Create execution context
      const context = this.createExecutionContext(input);

      // Execute the program
      const result = this.executeProgram(ast, context);

      return {
        success: true,
        value: result.value,
        sideEffects: {
          output: context.output,
        },
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Compile and run in one step
   */
  compileAndRun(source: string, input: string[] = []): ExecutionResult {
    const compilation = this.compile(source);

    if (!compilation.success || !compilation.ast) {
      return {
        success: false,
        error: 'Compilation failed',
      };
    }

    return this.run(compilation.ast, input);
  }

  /**
   * Get the VM registry
   */
  getVMRegistry(): VMRegistry {
    return this.vmRegistry;
  }

  /**
   * Get the memory VM
   */
  getMemoryVM(): MemoryVM {
    return this.memoryVM;
  }

  /**
   * Get the function VM
   */
  getFunctionVM(): FunctionVM {
    return this.functionVM;
  }

  /**
   * Get the IO VM
   */
  getIOVM(): IOVM {
    return this.ioVM;
  }

  private initializeVMs(): void {
    // Register all VMs
    this.vmRegistry.register(new ArithmeticVM());
    this.vmRegistry.register(new ComparisonVM());
    this.vmRegistry.register(new ControlFlowVM());
    this.vmRegistry.register(this.functionVM);
    this.vmRegistry.register(this.memoryVM);
    this.vmRegistry.register(this.ioVM);
    this.vmRegistry.register(new TypeVM());
  }

  private createExecutionContext(input: string[]): ExecutionContext {
    const memory = this.memoryVM.getMemory();
    const callStack: CallStack = {
      frames: [],
      currentFrame: -1,

      push(frame): void {
        this.frames.push(frame);
        this.currentFrame = this.frames.length - 1;
      },

      pop() {
        return this.frames.pop();
      },

      getCurrent() {
        if (this.currentFrame < 0 || this.currentFrame >= this.frames.length) {
          return undefined;
        }
        return this.frames[this.currentFrame];
      },

      peek() {
        return this.frames[this.frames.length - 1];
      },
    };

    return {
      memory,
      callStack,
      output: [],
      input,
      scopeDepth: 0,
      inLoop: false,
      breakFlag: false,
      continueFlag: false,
      hasReturned: false,
    };
  }

  private executeProgram(ast: ASTNode, context: ExecutionContext): ExecutionResult {
    if (ast.type !== NodeType.PROGRAM) {
      return {
        success: false,
        error: 'Root node must be a PROGRAM',
      };
    }

    const program = ast as {
      declarations: ASTNode[];
      functions: ASTNode[];
    };

    // First, process all function declarations
    for (const func of program.functions) {
      const result = this.vmRegistry.execute(func, context);
      if (!result.success) {
        return result;
      }
    }

    // Process global declarations
    for (const decl of program.declarations) {
      const result = this.vmRegistry.execute(decl, context);
      if (!result.success) {
        return result;
      }
    }

    // Look for main function and execute it
    const mainFunc = this.functionVM.getFunction('main');
    if (mainFunc) {
      // Execute main function
      const callNode: ASTNode = {
        type: NodeType.CALL_EXPR,
        line: 0,
        column: 0,
        callee: {
          type: NodeType.IDENTIFIER_EXPR,
          line: 0,
          column: 0,
          name: 'main',
        },
        arguments: [],
      };

      return this.vmRegistry.execute(callNode, context);
    }

    // No main function, just return success
    return {
      success: true,
    };
  }
}

// Alias for test compatibility
export { Compiler as CCompiler };