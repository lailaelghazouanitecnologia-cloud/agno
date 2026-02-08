/**
 * IOVM
 * Handles printf and scanf operations
 */

import { BaseMicroVM } from './interfaces.js';
import { ASTNode, NodeType, ExecutionContext, RuntimeValue, ValueType } from './types.js';

export class IOVM extends BaseMicroVM {
  readonly name = 'IOVM';

  canHandle(node: ASTNode): boolean {
    return node.type === NodeType.CALL_EXPR;
  }

  execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    const callNode = node as any;
    const functionName = callNode.functionName;

    if (functionName === 'printf') {
      return this.executePrintf(callNode, context);
    } else if (functionName === 'scanf') {
      return this.executeScanf(callNode, context);
    }

    // Not an I/O function, let other VMs handle it
    throw new Error(`IOVM cannot handle function: ${functionName}`);
  }

  /**
   * Execute printf
   */
  private executePrintf(node: any, context: ExecutionContext): RuntimeValue {
    if (node.arguments.length === 0) {
      throw new Error('printf requires at least a format string');
    }

    // Get format string
    const formatArg = this.evaluateExpression(node.arguments[0], context);
    let format: string;

    if (formatArg.type === ValueType.POINTER && typeof formatArg.value === 'string') {
      format = formatArg.value;
    } else {
      throw new Error('printf format string must be a string literal');
    }

    // Process format string and arguments
    let output = '';
    let argIndex = 1;
    let i = 0;

    while (i < format.length) {
      if (format[i] === '%' && i + 1 < format.length) {
        const specifier = format[i + 1];
        
        switch (specifier) {
          case 'd':
          case 'i':
            if (argIndex < node.arguments.length) {
              const arg = this.evaluateExpression(node.arguments[argIndex], context);
              output += this.formatInt(arg);
              argIndex++;
            } else {
              output += '%d';
            }
            i += 2;
            break;
          
          case 'f':
            if (argIndex < node.arguments.length) {
              const arg = this.evaluateExpression(node.arguments[argIndex], context);
              output += this.formatFloat(arg);
              argIndex++;
            } else {
              output += '%f';
            }
            i += 2;
            break;
          
          case 'c':
            if (argIndex < node.arguments.length) {
              const arg = this.evaluateExpression(node.arguments[argIndex], context);
              output += this.formatChar(arg);
              argIndex++;
            } else {
              output += '%c';
            }
            i += 2;
            break;
          
          case 's':
            if (argIndex < node.arguments.length) {
              const arg = this.evaluateExpression(node.arguments[argIndex], context);
              output += this.formatString(arg);
              argIndex++;
            } else {
              output += '%s';
            }
            i += 2;
            break;
          
          case '%':
            output += '%';
            i += 2;
            break;
          
          default:
            output += format[i];
            i++;
            break;
        }
      } else {
        output += format[i];
        i++;
      }
    }

    // Push output to context
    this.pushOutput(context, output);

    // Return number of characters printed
    return this.createValue(ValueType.INT, output.length);
  }

  /**
   * Execute scanf
   */
  private executeScanf(node: any, context: ExecutionContext): RuntimeValue {
    if (node.arguments.length === 0) {
      throw new Error('scanf requires at least a format string');
    }

    // Get format string
    const formatArg = this.evaluateExpression(node.arguments[0], context);
    let format: string;

    if (formatArg.type === ValueType.POINTER && typeof formatArg.value === 'string') {
      format = formatArg.value;
    } else {
      throw new Error('scanf format string must be a string literal');
    }

    // Count format specifiers
    let specifiers = 0;
    for (let i = 0; i < format.length; i++) {
      if (format[i] === '%' && i + 1 < format.length) {
        const specifier = format[i + 1];
        if (specifier === 'd' || specifier === 'f' || specifier === 'c' || specifier === 's') {
          specifiers++;
        }
      }
    }

    // For each specifier, read input and store in variable
    let itemsRead = 0;
    let argIndex = 1;

    for (let i = 0; i < format.length; i++) {
      if (format[i] === '%' && i + 1 < format.length) {
        const specifier = format[i + 1];
        
        if (argIndex >= node.arguments.length) {
          break;
        }

        // Get the variable to store into
        const varArg = node.arguments[argIndex];
        
        // Check if it's an address-of expression (pointer)
        if (varArg.type === NodeType.ADDRESS_OF_EXPR && varArg.operand.type === NodeType.IDENTIFIER_EXPR) {
          const varName = varArg.operand.name;
          
          // Read input
          if (context.input.length > 0) {
            const inputValue = context.input.shift()!;
            let value: RuntimeValue;

            switch (specifier) {
              case 'd':
              case 'i':
                value = this.createValue(ValueType.INT, parseInt(inputValue) || 0);
                break;
              
              case 'f':
                value = this.createValue(ValueType.FLOAT, parseFloat(inputValue) || 0.0);
                break;
              
              case 'c':
                value = this.createValue(ValueType.CHAR, inputValue.charCodeAt(0) || 0);
                break;
              
              case 's':
                value = this.createValue(ValueType.POINTER, inputValue);
                break;
              
              default:
                value = this.createValue(ValueType.INT, 0);
            }

            // Store in variable
            this.setVariable(varName, value, context);
            itemsRead++;
          }
        }

        argIndex++;
        i++;
      }
    }

    // Return number of items read
    return this.createValue(ValueType.INT, itemsRead);
  }

  /**
   * Format an integer value
   */
  private formatInt(value: RuntimeValue): string {
    switch (value.type) {
      case ValueType.INT:
        return String(value.value);
      case ValueType.FLOAT:
        return String(Math.trunc(value.value));
      case ValueType.CHAR:
        return String(value.value);
      default:
        return '0';
    }
  }

  /**
   * Format a float value
   */
  private formatFloat(value: RuntimeValue): string {
    switch (value.type) {
      case ValueType.FLOAT:
        return String(value.value);
      case ValueType.INT:
        return String(value.value) + '.0';
      default:
        return '0.0';
    }
  }

  /**
   * Format a character value
   */
  private formatChar(value: RuntimeValue): string {
    switch (value.type) {
      case ValueType.CHAR:
        return String.fromCharCode(value.value);
      case ValueType.INT:
        return String.fromCharCode(value.value);
      default:
        return '\0';
    }
  }

  /**
   * Format a string value
   */
  private formatString(value: RuntimeValue): string {
    if (value.type === ValueType.POINTER && typeof value.value === 'string') {
      return value.value;
    }
    return '';
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

    const leftNum = this.toNumber(left);
    const rightNum = this.toNumber(right);

    let result: number;
    switch (expr.operator) {
      case '+': result = leftNum + rightNum; break;
      case '-': result = leftNum - rightNum; break;
      case '*': result = leftNum * rightNum; break;
      case '/': result = leftNum / rightNum; break;
      case '%': result = leftNum % rightNum; break;
      default: throw new Error(`Unknown operator: ${expr.operator}`);
    }

    const useFloat = left.type === ValueType.FLOAT || right.type === ValueType.FLOAT;
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
}