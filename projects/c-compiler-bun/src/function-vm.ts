/**
 * FunctionVM - Handles function definitions, calls, returns, and call stack
 */

import {
  MicroVM,
  ASTNode,
  ExecutionContext,
  RuntimeValue,
  ValueType,
  FunctionDeclNode,
  CallExprNode,
  ReturnStmtNode,
  ParamNode,
  NodeType,
  StackFrame
} from "./interfaces.js";
import { MemoryVM } from "./memory-vm.js";

export class FunctionVM implements MicroVM {
  name = "FunctionVM";
  capabilities = ["function", "call", "return", "parameters"];

  private memory: MemoryVM;
  private functions: Map<string, FunctionDeclNode> = new Map();
  private callStack: StackFrame[] = [];

  constructor(memory: MemoryVM) {
    this.memory = memory;
  }

  /**
   * Execute a function-related node
   */
  public execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    switch (node.type) {
      case NodeType.FUNCTION_DECL:
        return this.registerFunction(node as FunctionDeclNode);
      case NodeType.CALL_EXPR:
        return this.executeCall(node as CallExprNode, context);
      case NodeType.RETURN_STMT:
        return this.executeReturn(node as ReturnStmtNode, context);
      default:
        throw new Error(`FunctionVM cannot execute node type: ${node.type}`);
    }
  }

  /**
   * Register a function definition
   */
  private registerFunction(node: FunctionDeclNode): void {
    this.functions.set(node.name, node);
  }

  /**
   * Execute a function call
   */
  private executeCall(node: CallExprNode, context: ExecutionContext): RuntimeValue {
    const func = this.functions.get(node.callee);

    if (!func) {
      // Check for built-in functions
      if (node.callee === "printf" || node.callee === "scanf") {
        // These are handled by IOVM
        return { type: ValueType.VOID, value: undefined };
      }

      throw new Error(`Undefined function: ${node.callee}`);
    }

    // Push new stack frame
    const frame = this.memory.pushFrame(func.name);
    this.callStack.push(frame);

    // Set up parameters
    for (let i = 0; i < func.params.length; i++) {
      const param = func.params[i];
      const argValue = node.args[i] || { type: ValueType.INT, value: 0 };

      // Store parameter in new frame
      this.memory.setVariable(param.name, argValue);
    }

    // Update context
    context.currentFunction = func.name;
    context.callDepth = (context.callDepth || 0) + 1;

    // Execute function body
    // This would delegate to the compiler in a full implementation
    // For now, we'll just return a default value

    // Pop stack frame
    this.memory.popFrame();
    this.callStack.pop();

    // Check for return value
    if (context.returnValue) {
      const returnValue = context.returnValue;
      context.returnValue = undefined;
      context.shouldReturn = false;
      return returnValue;
    }

    // Return default value based on return type
    return this.getDefaultValue(func.returnType);
  }

  /**
   * Execute a return statement
   */
  private executeReturn(node: ReturnStmtNode, context: ExecutionContext): void {
    if (node.value) {
      // Evaluate return value
      // This would delegate to the compiler in a full implementation
      context.returnValue = { type: ValueType.INT, value: 0 };
    } else {
      context.returnValue = { type: ValueType.VOID, value: undefined };
    }

    context.shouldReturn = true;
  }

  /**
   * Get a function by name
   */
  public getFunction(name: string): FunctionDeclNode | undefined {
    return this.functions.get(name);
  }

  /**
   * Check if a function exists
   */
  public hasFunction(name: string): boolean {
    return this.functions.has(name);
  }

  /**
   * Get all registered functions
   */
  public getFunctions(): Map<string, FunctionDeclNode> {
    return this.functions;
  }

  /**
   * Get current call stack depth
   */
  public getCallDepth(): number {
    return this.callStack.length;
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
      case "void":
        return { type: ValueType.VOID, value: undefined };
      default:
        return { type: ValueType.VOID, value: undefined };
    }
  }

  /**
   * Validate a function node
   */
  public validate(node: ASTNode): boolean {
    switch (node.type) {
      case NodeType.FUNCTION_DECL:
        return this.validateFunctionDecl(node as FunctionDeclNode);
      case NodeType.CALL_EXPR:
        return this.validateCall(node as CallExprNode);
      case NodeType.RETURN_STMT:
        return this.validateReturn(node as ReturnStmtNode);
      default:
        return false;
    }
  }

  /**
   * Validate function declaration
   */
  private validateFunctionDecl(node: FunctionDeclNode): boolean {
    // Check for duplicate function names
    if (this.functions.has(node.name)) {
      return false;
    }

    // Check that body is present
    if (!node.body) {
      return false;
    }

    return true;
  }

  /**
   * Validate function call
   */
  private validateCall(node: CallExprNode): boolean {
    const func = this.functions.get(node.callee);

    if (!func) {
      // Built-in functions are allowed
      if (node.callee === "printf" || node.callee === "scanf") {
        return true;
      }
      return false;
    }

    // Check argument count
    if (node.args.length !== func.params.length) {
      return false;
    }

    return true;
  }

  /**
   * Validate return statement
   */
  private validateReturn(node: ReturnStmtNode): boolean {
    // Return statements are always valid
    // Type checking would happen in a full implementation
    return true;
  }

  /**
   * Reset function registry (for testing)
   */
  public reset(): void {
    this.functions.clear();
    this.callStack = [];
  }
}