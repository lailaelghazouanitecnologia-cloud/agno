/**
 * FunctionVM - Handles function definitions, calls, returns, and call stack
 */

import { BaseVM } from './base-vm.js';
import { ASTNode, ExecutionContext, ExecutionResult, ValueType, CallStack, CallFrame } from '../core/interfaces.js';
import { NodeType } from '../core/types.js';

/**
 * Function signature information
 */
interface FunctionInfo {
  name: string;
  returnType: string;
  parameters: { name: string; type: string }[];
  body?: ASTNode;
}

/**
 * FunctionVM - handles function management
 */
export class FunctionVM extends BaseVM {
  readonly name = 'function';
  
  private functions: Map<string, FunctionInfo> = new Map();
  private callStack: CallStack;

  constructor() {
    super();
    this.callStack = this.createCallStack();
  }

  initialize?(context?: unknown): void {
    this.functions.clear();
    this.callStack = this.createCallStack();
  }

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    try {
      switch (node.type) {
        case NodeType.FUNCTION_DEF:
          return this.defineFunction(node, context);

        case NodeType.FUNCTION_DECL:
          return this.declareFunction(node, context);

        case NodeType.CALL_EXPR:
          return this.callFunction(node, context);

        case NodeType.RETURN_STMT:
          return this.handleReturn(node, context);

        default:
          return {
            success: false,
            error: `FunctionVM cannot handle node type: ${node.type}`,
          };
      }
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  canHandle(node: ASTNode): boolean {
    return (
      node.type === NodeType.FUNCTION_DEF ||
      node.type === NodeType.FUNCTION_DECL ||
      node.type === NodeType.CALL_EXPR ||
      node.type === NodeType.RETURN_STMT
    );
  }

  private defineFunction(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const func = node as {
      returnType: string;
      name: string;
      parameters: ASTNode[];
      body: ASTNode;
    };

    const parameters = func.parameters.map((param: ASTNode) => ({
      name: (param as { name: string }).name,
      type: (param as { paramType: string }).paramType,
    }));

    this.functions.set(func.name, {
      name: func.name,
      returnType: func.returnType,
      parameters,
      body: func.body,
    });

    return { success: true };
  }

  private declareFunction(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const func = node as {
      returnType: string;
      name: string;
      parameters: ASTNode[];
    };

    const parameters = func.parameters.map((param: ASTNode) => ({
      name: (param as { name: string }).name,
      type: (param as { paramType: string }).paramType,
    }));

    this.functions.set(func.name, {
      name: func.name,
      returnType: func.returnType,
      parameters,
    });

    return { success: true };
  }

  private callFunction(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const call = node as {
      callee: ASTNode;
      arguments: ASTNode[];
    };

    // Get function name
    let functionName: string;
    if (call.callee.type === NodeType.IDENTIFIER_EXPR) {
      functionName = (call.callee as { name: string }).name;
    } else {
      return {
        success: false,
        error: 'Only direct function calls are supported',
      };
    }

    // Get function info
    const funcInfo = this.functions.get(functionName);
    if (!funcInfo) {
      return {
        success: false,
        error: `Undefined function: ${functionName}`,
      };
    }

    if (!funcInfo.body) {
      return {
        success: false,
        error: `Function ${functionName} is declared but not defined`,
      };
    }

    // Evaluate arguments
    const args: ValueType[] = [];
    for (const arg of call.arguments) {
      const argResult = this.evaluateArgument(arg, context);
      if (!argResult.success) {
        return argResult;
      }
      args.push(argResult.value!);
    }

    // Create new call frame
    const frame: CallFrame = {
      functionName,
      locals: new Map(),
      parameters: new Map(),
    };

    // Bind parameters
    for (let i = 0; i < funcInfo.parameters.length; i++) {
      const param = funcInfo.parameters[i];
      const argValue = args[i] !== undefined ? args[i] : null;
      frame.parameters.set(param.name, argValue);
    }

    // Push call frame
    this.callStack.push(frame);

    // Create new execution context for function
    const functionContext: ExecutionContext = {
      ...context,
      memory: {
        ...context.memory,
        frames: [...context.memory.frames],
        currentFrame: context.memory.frames.length,
      },
      callStack: this.callStack,
      hasReturned: false,
      returnValue: undefined,
    };

    // Push new memory frame
    functionContext.memory.pushFrame();

    // Copy parameters to local memory
    for (const [name, value] of frame.parameters) {
      functionContext.memory.allocate(name, 'auto', value);
    }

    // Execute function body
    let result: ExecutionResult;
    try {
      result = this.executeStatement(funcInfo.body, functionContext);
    } finally {
      // Pop memory frame
      functionContext.memory.popFrame();
      
      // Pop call frame
      this.callStack.pop();
    }

    // Return result or return value
    if (functionContext.hasReturned) {
      return {
        success: true,
        value: functionContext.returnValue,
      };
    }

    return result;
  }

  private handleReturn(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const returnStmt = node as {
      value: ASTNode | null;
    };

    let returnValue: ValueType = undefined;
    if (returnStmt.value) {
      const result = this.evaluateArgument(returnStmt.value, context);
      if (!result.success) {
        return result;
      }
      returnValue = result.value;
    }

    context.hasReturned = true;
    context.returnValue = returnValue;

    return {
      success: true,
      value: returnValue,
    };
  }

  private evaluateArgument(node: ASTNode, context: ExecutionContext): ExecutionResult {
    // Handle literals
    if (node.type === NodeType.INTEGER_LITERAL) {
      return { success: true, value: (node as { value: number }).value };
    }
    if (node.type === NodeType.FLOAT_LITERAL) {
      return { success: true, value: (node as { value: number }).value };
    }
    if (node.type === NodeType.CHAR_LITERAL) {
      return { success: true, value: (node as { value: string }).value };
    }
    if (node.type === NodeType.STRING_LITERAL) {
      return { success: true, value: (node as { value: string }).value };
    }

    // Handle identifiers
    if (node.type === NodeType.IDENTIFIER_EXPR) {
      const name = (node as { name: string }).name;
      const value = context.memory.get(name);
      if (value === undefined) {
        return {
          success: false,
          error: `Undefined variable: ${name}`,
        };
      }
      return { success: true, value };
    }

    // For other expressions, we'd need to delegate to appropriate VMs
    // This is a placeholder
    return {
      success: false,
      error: `Cannot evaluate argument node type: ${node.type}`,
    };
  }

  private executeStatement(node: ASTNode, context: ExecutionContext): ExecutionResult {
    // Handle compound statements
    if (node.type === NodeType.COMPOUND_STMT) {
      const compound = node as { statements: ASTNode[] };
      for (const stmt of compound.statements) {
        const result = this.executeStatement(stmt, context);
        if (!result.success) {
          return result;
        }
        if (context.hasReturned) {
          break;
        }
      }
      return { success: true };
    }

    // For other statement types, we'd need to delegate
    // This is a placeholder
    return { success: true };
  }

  private createCallStack(): CallStack {
    return {
      frames: [],
      currentFrame: -1,

      push(frame: CallFrame): void {
        this.frames.push(frame);
        this.currentFrame = this.frames.length - 1;
      },

      pop(): CallFrame | undefined {
        return this.frames.pop();
      },

      getCurrent(): CallFrame | undefined {
        if (this.currentFrame < 0 || this.currentFrame >= this.frames.length) {
          return undefined;
        }
        return this.frames[this.currentFrame];
      },

      peek(): CallFrame | undefined {
        return this.frames[this.frames.length - 1];
      },
    };
  }

  /**
   * Get all defined functions
   */
  getFunctions(): Map<string, FunctionInfo> {
    return new Map(this.functions);
  }

  /**
   * Get a function by name
   */
  getFunction(name: string): FunctionInfo | undefined {
    return this.functions.get(name);
  }

  /**
   * Check if a function is defined
   */
  hasFunction(name: string): boolean {
    return this.functions.has(name);
  }
}