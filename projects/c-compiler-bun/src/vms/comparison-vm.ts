/**
 * ComparisonVM
 * Handles comparison operations: <, >, <=, >=, ==, !=
 */

import { BaseMicroVM } from './interfaces.js';
import { ASTNode, NodeType, ExecutionContext, RuntimeValue, ValueType, TokenType } from './types.js';

export class ComparisonVM extends BaseMicroVM {
  readonly name = 'ComparisonVM';

  canHandle(node: ASTNode): boolean {
    if (node.type !== NodeType.BINARY_EXPR) {
      return false;
    }

    const binaryNode = node as any;
    const operator = binaryNode.operator;

    return (
      operator === TokenType.LESS ||
      operator === TokenType.GREATER ||
      operator === TokenType.LESS_EQUAL ||
      operator === TokenType.GREATER_EQUAL ||
      operator === TokenType.EQUAL ||
      operator === TokenType.NOT_EQUAL
    );
  }

  execute(node: ASTNode, context: ExecutionContext): RuntimeValue {
    const binaryNode = node as any;
    const operator = binaryNode.operator;

    // Evaluate left and right operands
    const leftValue = this.evaluateOperand(binaryNode.left, context);
    const rightValue = this.evaluateOperand(binaryNode.right, context);

    // Perform the comparison
    const result = this.performComparison(operator, leftValue, rightValue);

    return this.createValue(ValueType.INT, result ? 1 : 0);
  }

  /**
   * Evaluate an operand
   */
  private evaluateOperand(operand: any, context: ExecutionContext): RuntimeValue {
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
        // Check if it's an arithmetic operation
        const isArithmetic = 
          operand.operator === TokenType.PLUS ||
          operand.operator === TokenType.MINUS ||
          operand.operator === TokenType.STAR ||
          operand.operator === TokenType.SLASH ||
          operand.operator === TokenType.PERCENT;
        
        if (isArithmetic) {
          // Would be handled by ArithmeticVM, but we need to evaluate it here
          // For simplicity, we'll do basic evaluation
          const left = this.evaluateOperand(operand.left, context);
          const right = this.evaluateOperand(operand.right, context);
          return this.evaluateBinaryOp(operand.operator, left, right);
        }
        throw new Error(`Nested comparison not supported: ${operand.operator}`);
      
      default:
        throw new Error(`Cannot evaluate operand of type: ${operand.type}`);
    }
  }

  /**
   * Evaluate a binary operation (for nested arithmetic)
   */
  private evaluateBinaryOp(operator: TokenType, left: RuntimeValue, right: RuntimeValue): RuntimeValue {
    const leftNum = this.toNumber(left);
    const rightNum = this.toNumber(right);
    const useFloat = left.type === ValueType.FLOAT || right.type === ValueType.FLOAT;

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
        result = leftNum / rightNum;
        break;
      case TokenType.PERCENT:
        result = leftNum % rightNum;
        break;
      default:
        throw new Error(`Unknown operator: ${operator}`);
    }

    if (useFloat) {
      return this.createValue(ValueType.FLOAT, result);
    } else {
      return this.createValue(ValueType.INT, Math.trunc(result));
    }
  }

  /**
   * Perform the comparison
   */
  private performComparison(
    operator: TokenType,
    left: RuntimeValue,
    right: RuntimeValue
  ): boolean {
    const leftNum = this.toNumber(left);
    const rightNum = this.toNumber(right);

    switch (operator) {
      case TokenType.LESS:
        return leftNum < rightNum;
      
      case TokenType.GREATER:
        return leftNum > rightNum;
      
      case TokenType.LESS_EQUAL:
        return leftNum <= rightNum;
      
      case TokenType.GREATER_EQUAL:
        return leftNum >= rightNum;
      
      case TokenType.EQUAL:
        return leftNum === rightNum;
      
      case TokenType.NOT_EQUAL:
        return leftNum !== rightNum;
      
      default:
        throw new Error(`Unknown comparison operator: ${operator}`);
    }
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
      
      case ValueType.POINTER:
        return value.value as number;
      
      default:
        throw new Error(`Cannot convert ${value.type} to number for comparison`);
    }
  }
}