/**
 * Main Compiler - Orchestrates all components to compile and run C code
 */

import { Lexer } from "./lexer.js";
import { Parser } from "./parser.js";
import { MicroVMRegistry } from "./microvm-registry.js";
import {
  ProgramNode,
  ASTNode,
  ExecutionContext,
  RuntimeValue,
  ValueType,
  CompileResult,
  CompilerOptions,
  CompilerError,
  FunctionDeclNode,
  CompoundStmtNode,
  StmtNode,
  ExprStmtNode,
  VarDeclNode,
  NodeType,
  BinaryExprNode,
  IdentifierNode,
  NumberNode
} from "./interfaces.js";

export class CCompiler {
  private registry: MicroVMRegistry;
  private options: CompilerOptions;

  constructor(options: CompilerOptions = {}) {
    this.options = {
      debug: false,
      verbose: false,
      outputAST: false,
      outputTokens: false,
      strictTypes: true,
      ...options
    };

    this.registry = new MicroVMRegistry();
  }

  /**
   * Compile and run C source code
   */
  public compileAndRun(source: string): CompileResult {
    const startTime = performance.now();
    const errors: string[] = [];
    const warnings: string[] = [];

    try {
      // Step 1: Lexical analysis
      if (this.options.verbose) {
        console.log("[Compiler] Lexical analysis...");
      }

      const lexer = new Lexer(source);
      const tokens = lexer.tokenize();

      if (this.options.outputTokens) {
        console.log("[Compiler] Tokens:");
        console.log(JSON.stringify(tokens, null, 2));
      }

      // Step 2: Parsing
      if (this.options.verbose) {
        console.log("[Compiler] Parsing...");
      }

      const parser = new Parser(tokens);
      const ast = parser.parse();

      if (this.options.outputAST) {
        console.log("[Compiler] AST:");
        console.log(JSON.stringify(ast, null, 2));
      }

      // Step 3: Register functions
      if (this.options.verbose) {
        console.log("[Compiler] Registering functions...");
      }

      this.registerFunctions(ast);

      // Step 4: Execute
      if (this.options.verbose) {
        console.log("[Compiler] Executing...");
      }

      const result = this.execute(ast);

      const executionTime = performance.now() - startTime;

      return {
        success: true,
        output: result,
        errors,
        warnings,
        executionTime
      };

    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      errors.push(errorMessage);

      return {
        success: false,
        errors,
        warnings,
        executionTime: performance.now() - startTime
      };
    }
  }

  /**
   * Register all functions from AST
   */
  private registerFunctions(ast: ProgramNode): void {
    for (const decl of ast.declarations) {
      if (decl.type === NodeType.FUNCTION_DECL) {
        this.registry.function.execute(decl, {});
      }
    }
  }

  /**
   * Execute the AST
   */
  private execute(ast: ProgramNode): RuntimeValue | undefined {
    // Find and execute main function
    const mainFunc = this.findMainFunction(ast);

    if (!mainFunc) {
      throw new CompilerError("No main() function defined");
    }

    // Create execution context
    const context: ExecutionContext = {
      currentFunction: "main",
      callDepth: 0
    };

    // Execute main function
    return this.executeFunction(mainFunc, context);
  }

  /**
   * Find main function in AST
   */
  private findMainFunction(ast: ProgramNode): FunctionDeclNode | undefined {
    for (const decl of ast.declarations) {
      if (decl.type === NodeType.FUNCTION_DECL && decl.name === "main") {
        return decl;
      }
    }
    return undefined;
  }

  /**
   * Execute a function
   */
  private executeFunction(func: FunctionDeclNode, context: ExecutionContext): RuntimeValue {
    // Push new stack frame
    this.registry.memory.pushFrame(func.name);

    try {
      // Execute function body
      const result = this.executeCompoundStmt(func.body, context);

      // Check for return value
      if (context.returnValue) {
        return context.returnValue;
      }

      // Default return value
      return { type: ValueType.INT, value: 0 };

    } finally {
      // Pop stack frame
      this.registry.memory.popFrame();
    }
  }

  /**
   * Execute a compound statement (block)
   */
  private executeCompoundStmt(node: CompoundStmtNode, context: ExecutionContext): RuntimeValue | undefined {
    for (const stmt of node.statements) {
      const result = this.executeStatement(stmt, context);

      // Check for return
      if (context.shouldReturn) {
        return result;
      }

      // Check for break
      if (context.breakLoop) {
        break;
      }

      // Check for continue
      if (context.continueLoop) {
        continue;
      }
    }

    return undefined;
  }

  /**
   * Execute a statement
   */
  private executeStatement(stmt: StmtNode, context: ExecutionContext): RuntimeValue | undefined {
    switch (stmt.type) {
      case NodeType.VAR_DECL:
        return this.executeVarDecl(stmt as VarDeclNode, context);

      case NodeType.EXPR_STMT:
        return this.executeExprStmt(stmt as ExprStmtNode, context);

      case NodeType.IF_STMT:
      case NodeType.WHILE_STMT:
      case NodeType.FOR_STMT:
        return this.registry.controlFlow.execute(stmt, context);

      case NodeType.RETURN_STMT:
        return this.registry.function.execute(stmt, context);

      case NodeType.COMPOUND_STMT:
        return this.executeCompoundStmt(stmt as CompoundStmtNode, context);

      default:
        // Try dispatching through registry
        return this.registry.dispatch(stmt, context);
    }
  }

  /**
   * Execute variable declaration
   */
  private executeVarDecl(node: VarDeclNode, context: ExecutionContext): RuntimeValue {
    let value: RuntimeValue;

    if (node.init) {
      value = this.evaluateExpression(node.init, context);
    } else {
      // Default value based on type
      value = this.getDefaultValue(node.varType);
    }

    this.registry.memory.declareVariable(node.name, value);
    return value;
  }

  /**
   * Execute expression statement
   */
  private executeExprStmt(node: ExprStmtNode, context: ExecutionContext): RuntimeValue | undefined {
    // Check for assignment
    if (node.expression.type === NodeType.BINARY_EXPR) {
      const binary = node.expression as BinaryExprNode;
      if (binary.operator === "=") {
        return this.executeAssignment(binary, context);
      }
    }

    // Just evaluate the expression
    return this.evaluateExpression(node.expression, context);
  }

  /**
   * Execute assignment
   */
  private executeAssignment(node: BinaryExprNode, context: ExecutionContext): RuntimeValue {
    // Evaluate right side
    const value = this.evaluateExpression(node.right, context);

    // Get target variable name
    if (node.left.type === NodeType.IDENTIFIER) {
      const varName = (node.left as IdentifierNode).name;
      this.registry.memory.setVariable(varName, value);
    }

    return value;
  }

  /**
   * Evaluate an expression
   */
  private evaluateExpression(expr: ASTNode, context: ExecutionContext): RuntimeValue {
    switch (expr.type) {
      case NodeType.NUMBER:
        const numNode = expr as NumberNode;
        if (Number.isInteger(numNode.value)) {
          return { type: ValueType.INT, value: numNode.value };
        } else {
          return { type: ValueType.FLOAT, value: numNode.value };
        }

      case NodeType.IDENTIFIER:
        const identNode = expr as IdentifierNode;
        const value = this.registry.memory.getVariable(identNode.name);
        if (value === undefined) {
          throw new CompilerError(`Undefined variable: ${identNode.name}`);
        }
        return value;

      case NodeType.BINARY_EXPR:
        return this.registry.dispatch(expr, context) as RuntimeValue;

      case NodeType.UNARY_EXPR:
        return this.registry.dispatch(expr, context) as RuntimeValue;

      case NodeType.CALL_EXPR:
        return this.executeCall(expr, context);

      default:
        throw new CompilerError(`Cannot evaluate expression type: ${expr.type}`);
    }
  }

  /**
   * Execute function call
   */
  private executeCall(node: ASTNode, context: ExecutionContext): RuntimeValue {
    // Check for built-in functions
    const callNode = node as any;
    const callee = callNode.callee;

    if (callee === "printf" || callee === "scanf") {
      return this.registry.io.execute(node, context) as RuntimeValue;
    }

    // User-defined function
    const func = this.registry.function.getFunction(callee);
    if (!func) {
      throw new CompilerError(`Undefined function: ${callee}`);
    }

    // Evaluate arguments
    const args: RuntimeValue[] = [];
    for (const arg of callNode.args) {
      args.push(this.evaluateExpression(arg, context));
    }

    // Create new context for function call
    const funcContext: ExecutionContext = {
      currentFunction: callee,
      callDepth: (context.callDepth || 0) + 1
    };

    // Push stack frame
    this.registry.memory.pushFrame(callee);

    try {
      // Set up parameters
      for (let i = 0; i < func.params.length; i++) {
        const param = func.params[i];
        const argValue = args[i] || { type: ValueType.INT, value: 0 };
        this.registry.memory.setVariable(param.name, argValue);
      }

      // Execute function body
      this.executeCompoundStmt(func.body, funcContext);

      // Get return value
      if (funcContext.returnValue) {
        return funcContext.returnValue;
      }

      // Default return
      return { type: ValueType.INT, value: 0 };

    } finally {
      // Pop stack frame
      this.registry.memory.popFrame();
    }
  }

  /**
   * Get default value for a type
   */
  private getDefaultValue(type: string): RuntimeValue {
    switch (type) {
      case "int":
        return { type: ValueType.INT, value: 0 };
      case "float":
        return { type: ValueType.FLOAT, value: 0.0 };
      case "char":
        return { type: ValueType.CHAR, value: "\0" };
      default:
        return { type: ValueType.VOID, value: undefined };
    }
  }

  /**
   * Get compiler options
   */
  public getOptions(): CompilerOptions {
    return { ...this.options };
  }

  /**
   * Set compiler options
   */
  public setOptions(options: Partial<CompilerOptions>): void {
    this.options = { ...this.options, ...options };
  }

  /**
   * Get the VM registry
   */
  public getRegistry(): MicroVMRegistry {
    return this.registry;
  }

  /**
   * Reset compiler state
   */
  public reset(): void {
    this.registry.reset();
  }
}