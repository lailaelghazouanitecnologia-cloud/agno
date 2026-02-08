/**
 * ArithmeticVM
 * Handles arithmetic operations: +, -, *, /, %
 */

import { BaseMicroVM } from './interfaces.js';
import { ASTNode, NodeType, ExecutionContext, RuntimeValue, ValueType, TokenType } from './types.js';

export class ArithmeticVM extends BaseMicroVM {
  readonly name = 'ArithmeticVM';

  canHandle(node: ASTNode): boolean {
    return node.type === NodeType.BINARY_EXPR;
  }

  execute(node: ASTNode, context: ExecutionContext): RuntimeValue {
    const binaryNode = node as any;
    const operator = binaryNode.operator;

    // Only handle arithmetic operators
    if (
      operator !== TokenType.PLUS &&
      operator !== TokenType.MINUS &&
      operator !== TokenType.STAR &&
      operator !== TokenType.SLASH &&
      operator !== TokenType.PERCENT
    ) {
      throw new Error(`ArithmeticVM cannot handle operator: ${operator}`);
    }

    // Evaluate left and right operands
    const leftValue = this.evaluateOperand(binaryNode.left, context);
    const rightValue = this.evaluateOperand(binaryNode.right, context);

    // Perform the operation
    return this.performOperation(operator, leftValue, rightValue);
  }

  /**
   * Evaluate an operand (could be a literal, identifier, or another expression)
   */
  private evaluateOperand(operand: any, context: ExecutionContext): RuntimeValue {
    // If it's already a runtime value, return it
    if (operand.type && operand.value !== undefined) {
      return operand as RuntimeValue;
    }

    // Handle different node types
    switch (operand.type) {
      case NodeType.INT_LITERAL:
        return this.createValue(ValueType.INT, operand.value);
      
      case NodeType.FLOAT_LITERAL:
        return this.createValue(ValueType.FLOAT, operand.value);
      
      case NodeType.CHAR_LITERAL:
        return this.createValue(ValueType.CHAR, operand.value.charCodeAt(0));
      
      case NodeType.IDENTIFIER_EXPR:
        const varValue = this.getVariable(operand.name, context);
        if (!varValue) {
          throw new Error(`Undefined variable: ${operand.name}`);
        }
        return varValue;
      
      case NodeType.BINARY_EXPR:
        // Recursively evaluate binary expressions
        return this.execute(operand, context);
      
      case NodeType.UNARY_EXPR:
        // Handle unary minus
        if (operand.operator === TokenType.MINUS) {
          const innerValue = this.evaluateOperand(operand.operand, context);
          if (innerValue.type === ValueType.INT) {
            return this.createValue(ValueType.INT, -innerValue.value);
          } else if (innerValue.type === ValueType.FLOAT) {
            return this.createValue(ValueType.FLOAT, -innerValue.value);
          }
        }
        throw new Error(`Unsupported unary operator in arithmetic: ${operand.operator}`);
      
      case NodeType.CALL_EXPR:
        // This would be handled by FunctionVM
        throw new Error('Function calls in arithmetic expressions not yet supported');
      
      case NodeType.ARRAY_ACCESS_EXPR:
        // This would be handled by MemoryVM
        throw new Error('Array access in arithmetic expressions not yet supported');
      
      default:
        throw new Error(`Cannot evaluate operand of type: ${operand.type}`);
    }
  }

  /**
   * Perform the arithmetic operation
   */
  private performOperation(
    operator: TokenType,
    left: RuntimeValue,
    right: RuntimeValue
  ): RuntimeValue {
    // Type promotion: if either operand is float, result is float
    const useFloat = left.type === ValueType.FLOAT || right.type === ValueType.FLOAT;

    // Convert to numbers
    const leftNum = this.toNumber(left);
    const rightNum = this.toNumber(right);

    let result: number;

    switch (operator) {
      case TokenType.PLUS:
        result = leftNum + rightNum;
        break;
      
      case TokenType.MINUS:
        result = leftNum - rightNum;
        break;
      
      case TokenType.STAR:
        result = leftNum * rightNum;
        break;
      
      case TokenType.SLASH:
        if (rightNum === 0) {
          throw new Error('Division by zero');
        }
        result = leftNum / rightNum;
        break;
      
      case TokenType.PERCENT:
        if (rightNum === 0) {
          throw new Error('Modulo by zero');
        }
        result = leftNum % rightNum;
        break;
      
      default:
        throw new Error(`Unknown arithmetic operator: ${operator}`);
    }

    // Return result with appropriate type
    if (useFloat) {
      return this.createValue(ValueType.FLOAT, result);
    } else {
      return this.createValue(ValueType.INT, Math.trunc(result));
    }
  }

  /**
   * Convert a runtime value to a number
   */
  private toNumber(value: RuntimeValue): number {
    switch (value.type) {
      case ValueType.INT:
      case ValueType.FLOAT:
        return Number(value.value);
      
      case ValueType.CHAR:
        return value.value;
      
      case ValueType.POINTER:
        return value.value as number;
      
      default:
        throw new Error(`Cannot convert ${value.type} to number`);
    }
  }
}