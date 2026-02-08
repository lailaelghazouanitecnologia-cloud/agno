/**
 * FunctionVM
 * Handles function definitions, calls, returns, and call stack
 */

import { BaseMicroVM } from './interfaces.js';
import { ASTNode, NodeType, ExecutionContext, RuntimeValue, ValueType, StackFrame } from './types.js';

export class FunctionVM extends BaseMicroVM {
  readonly name = 'FunctionVM';

  // Store function definitions
  private functions: Map<string, any>;

  constructor() {
    super();
    this.functions = new Map();
  }

  canHandle(node: ASTNode): boolean {
    return (
      node.type === NodeType.FUNCTION_DEF ||
      node.type === NodeType.FUNCTION_DECL ||
      node.type === NodeType.CALL_EXPR ||
      node.type === NodeType.RETURN_STMT
    );
  }

  execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    switch (node.type) {
      case NodeType.FUNCTION_DEF:
        return this.defineFunction(node as any, context);
      
      case NodeType.FUNCTION_DECL:
        // Just store the declaration (for forward references)
        this.defineFunction(node as any, context);
        return;
      
      case NodeType.CALL_EXPR:
        return this.callFunction(node as any, context);
      
      case NodeType.RETURN_STMT:
        return this.handleReturn(node as any, context);
      
      default:
        throw new Error(`FunctionVM cannot handle node type: ${node.type}`);
    }
  }

  /**
   * Define a function
   */
  private defineFunction(node: any, context: ExecutionContext): void {
    this.functions.set(node.name, node);
  }

  /**
   * Call a function
   */
  private callFunction(node: any, context: ExecutionContext): RuntimeValue {
    const functionName = node.functionName;
    const funcDef = this.functions.get(functionName);

    if (!funcDef) {
      throw new Error(`Undefined function: ${functionName}`);
    }

    // Evaluate arguments
    const args: RuntimeValue[] = [];
    for (const arg of node.arguments) {
      args.push(this.evaluateExpression(arg, context));
    }

    // Handle built-in functions
    if (this.isBuiltinFunction(functionName)) {
      return this.executeBuiltin(functionName, args, context);
    }

    // Create new stack frame
    const newFrame: StackFrame = {
      functionName,
      locals: new Map(),
      parameters: [],
      returnAddress: context.pc
    };

    // Set up parameters
    for (let i = 0; i < funcDef.parameters.length; i++) {
      const param = funcDef.parameters[i];
      const argValue = args[i] || this.createValue(ValueType.INT, 0);
      
      newFrame.parameters.push(argValue);
      newFrame.locals.set(param.name, argValue);
    }

    // Push new frame
    context.stack.push(newFrame);
    context.currentFrame = context.stack.length - 1;

    // Clear return state
    const prevShouldReturn = context.shouldReturn;
    const prevReturnValue = context.returnValue;
    context.shouldReturn = false;
    context.returnValue = undefined;

    // Execute function body
    this.executeCompoundStatement(funcDef.body, context);

    // Get return value
    const returnValue = context.returnValue || this.createValue(ValueType.VOID, undefined);

    // Pop stack frame
    context.stack.pop();
    context.currentFrame = context.stack.length - 1;

    // Restore return state
    context.shouldReturn = prevShouldReturn;
    context.returnValue = prevReturnValue;

    return returnValue;
  }

  /**
   * Handle a return statement
   */
  private handleReturn(node: any, context: ExecutionContext): void {
    context.shouldReturn = true;
    
    if (node.value) {
      context.returnValue = this.evaluateExpression(node.value, context);
    } else {
      context.returnValue = this.createValue(ValueType.VOID, undefined);
    }
  }

  /**
   * Execute a compound statement (function body)
   */
  private executeCompoundStatement(node: any, context: ExecutionContext): void {
    for (const stmt of node.statements) {
      this.executeStatement(stmt, context);

      // Check for return
      if (context.shouldReturn) {
        return;
      }
    }
  }

  /**
   * Execute a statement
   */
  private executeStatement(stmt: any, context: ExecutionContext): void {
    switch (stmt.type) {
      case NodeType.COMPOUND_STMT:
        this.executeCompoundStatement(stmt, context);
        break;
      
      case NodeType.VAR_DECL:
        // Would be handled by MemoryVM
        this.executeVarDecl(stmt, context);
        break;
      
      case NodeType.EXPR_STMT:
        this.evaluateExpression(stmt.expression, context);
        break;
      
      case NodeType.IF_STMT:
      case NodeType.WHILE_STMT:
      case NodeType.FOR_STMT:
        // These would be handled by ControlFlowVM
        // For now, we'll skip (simplified)
        break;
      
      case NodeType.RETURN_STMT:
        this.handleReturn(stmt, context);
        break;
      
      case NodeType.ASSIGN_EXPR:
        const value = this.evaluateExpression(stmt.value, context);
        if (stmt.target.type === NodeType.IDENTIFIER_EXPR) {
          this.setVariable(stmt.target.name, value, context);
        }
        break;
      
      default:
        throw new Error(`Unknown statement type: ${stmt.type}`);
    }
  }

  /**
   * Execute a variable declaration
   */
  private executeVarDecl(node: any, context: ExecutionContext): void {
    let value: RuntimeValue;

    if (node.init) {
      value = this.evaluateExpression(node.init, context);
    } else {
      // Default initialization
      value = this.createValue(ValueType.INT, 0);
    }

    this.setVariable(node.name, value, context);
  }

  /**
   * Evaluate an expression
   */
  private evaluateExpression(expr: any, context: ExecutionContext): RuntimeValue {
    switch (expr.type) {
      case NodeType.INT_LITERAL:
        return this.createValue(ValueType.INT, expr.value);
      
      case NodeType.FLOAT_LITERAL:
        return this.createValue(ValueType.FLOAT, expr.value);
      
      case NodeType.CHAR_LITERAL:
        return this.createValue(ValueType.CHAR, expr.value.charCodeAt(0));
      
      case NodeType.STRING_LITERAL:
        return this.createValue(ValueType.POINTER, expr.value);
      
      case NodeType.IDENTIFIER_EXPR:
        const varValue = this.getVariable(expr.name, context);
        if (!varValue) {
          throw new Error(`Undefined variable: ${expr.name}`);
        }
        return varValue;
      
      case NodeType.BINARY_EXPR:
        return this.evaluateBinaryExpr(expr, context);
      
      case NodeType.UNARY_EXPR:
        return this.evaluateUnaryExpr(expr, context);
      
      case NodeType.CALL_EXPR:
        return this.callFunction(expr, context);
      
      case NodeType.ASSIGN_EXPR:
        const value = this.evaluateExpression(expr.value, context);
        if (expr.target.type === NodeType.IDENTIFIER_EXPR) {
          this.setVariable(expr.target.name, value, context);
        }
        return value;
      
      default:
        throw new Error(`Cannot evaluate expression of type: ${expr.type}`);
    }
  }

  /**
   * Evaluate a binary expression
   */
  private evaluateBinaryExpr(expr: any, context: ExecutionContext): RuntimeValue {
    const left = this.evaluateExpression(expr.left, context);
    const right = this.evaluateExpression(expr.right, context);

    // This would be handled by ArithmeticVM or ComparisonVM
    // For simplicity, we'll do basic evaluation
    const leftNum = this.toNumber(left);
    const rightNum = this.toNumber(right);

    // Check if it's a comparison operator
    const isComparison = 
      expr.operator === '<' ||
      expr.operator === '>' ||
      expr.operator === '<=' ||
      expr.operator === '>=' ||
      expr.operator === '==' ||
      expr.operator === '!=';

    if (isComparison) {
      let result: boolean;
      switch (expr.operator) {
        case '<': result = leftNum < rightNum; break;
        case '>': result = leftNum > rightNum; break;
        case '<=': result = leftNum <= rightNum; break;
        case '>=': result = leftNum >= rightNum; break;
        case '==': result = leftNum === rightNum; break;
        case '!=': result = leftNum !== rightNum; break;
        default: result = false;
      }
      return this.createValue(ValueType.INT, result ? 1 : 0);
    }

    // Arithmetic operation
    const useFloat = left.type === ValueType.FLOAT || right.type === ValueType.FLOAT;
    let result: number;

    switch (expr.operator) {
      case '+': result = leftNum + rightNum; break;
      case '-': result = leftNum - rightNum; break;
      case '*': result = leftNum * rightNum; break;
      case '/': result = leftNum / rightNum; break;
      case '%': result = leftNum % rightNum; break;
      default: throw new Error(`Unknown operator: ${expr.operator}`);
    }

    if (useFloat) {
      return this.createValue(ValueType.FLOAT, result);
    } else {
      return this.createValue(ValueType.INT, Math.trunc(result));
    }
  }

  /**
   * Evaluate a unary expression
   */
  private evaluateUnaryExpr(expr: any, context: ExecutionContext): RuntimeValue {
    const operand = this.evaluateExpression(expr.operand, context);

    if (expr.operator === '-') {
      if (operand.type === ValueType.INT) {
        return this.createValue(ValueType.INT, -operand.value);
      } else if (operand.type === ValueType.FLOAT) {
        return this.createValue(ValueType.FLOAT, -operand.value);
      }
    }

    throw new Error(`Unsupported unary operator: ${expr.operator}`);
  }

  /**
   * Check if a function is built-in
   */
  private isBuiltinFunction(name: string): boolean {
    return name === 'printf' || name === 'scanf';
  }

  /**
   * Execute a built-in function
   */
  private executeBuiltin(name: string, args: RuntimeValue[], context: ExecutionContext): RuntimeValue {
    // These would be handled by IOVM
    // For now, we'll do basic handling
    if (name === 'printf') {
      const format = args[0]?.value || '';
      let output = format;
      
      // Simple format string replacement
      for (let i = 1; i < args.length; i++) {
        const arg = args[i];
        let valueStr: string;
        
        if (arg.type === ValueType.INT) {
          valueStr = String(arg.value);
        } else if (arg.type === ValueType.FLOAT) {
          valueStr = String(arg.value);
        } else if (arg.type === ValueType.CHAR) {
          valueStr = String.fromCharCode(arg.value);
        } else if (arg.type === ValueType.POINTER && typeof arg.value === 'string') {
          valueStr = arg.value;
        } else {
          valueStr = String(arg.value);
        }
        
        output = output.replace('%d', valueStr).replace('%f', valueStr).replace('%c', valueStr).replace('%s', valueStr);
      }
      
      this.pushOutput(context, output);
      return this.createValue(ValueType.INT, output.length);
    }

    if (name === 'scanf') {
      // Simplified scanf - just read from input
      // In a real implementation, this would parse the format string
      if (context.input.length > 0) {
        const inputValue = context.input.shift()!;
        return this.createValue(ValueType.INT, 1);
      }
      return this.createValue(ValueType.INT, 0);
    }

    throw new Error(`Unknown built-in function: ${name}`);
  }

  /**
   * Convert a runtime value to a number
   */
  private toNumber(value: RuntimeValue): number {
    switch (value.type) {
      case ValueType.INT:
      case ValueType.FLOAT:
      case ValueType.CHAR:
        return Number(value.value);
      default:
        throw new Error(`Cannot convert ${value.type} to number`);
    }
  }

  /**
   * Reset the VM
   */
  public reset(): void {
    this.functions.clear();
  }
}