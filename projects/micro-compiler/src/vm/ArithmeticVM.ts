import {
  MicroVM,
  ASTNode,
  ExecutionContext,
  ExecutionResult
} from '../types/MicroVM';
import {
  BinaryExpression,
  LiteralExpression
} from '../types/AST';
import { createValue, Value, ValueType } from '../types/Value';

/**
 * ArithmeticVM - Handles arithmetic operations: +, -, *, /, %
 */
export class ArithmeticVM implements MicroVM {
  getName(): string {
    return 'ArithmeticVM';
  }

  canHandle(node: ASTNode): boolean {
    if (node.type === 'BinaryExpression') {
      const expr = node as BinaryExpression;
      return ['+', '-', '*', '/', '%'].includes(expr.operator);
    }
    if (node.type === 'LiteralExpression') {
      const expr = node as LiteralExpression;
      return expr.value.type === ValueType.NUMBER;
    }
    return false;
  }

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    switch (node.type) {
      case 'BinaryExpression':
        return this.executeBinaryExpression(node as BinaryExpression, context);

      case 'LiteralExpression':
        return this.executeLiteralExpression(node as LiteralExpression, context);

      default:
        return {
          error: new Error(`ArithmeticVM cannot handle node type: ${node.type}`)
        };
    }
  }

  /**
   * Execute a binary arithmetic expression
   */
  private executeBinaryExpression(
    node: BinaryExpression,
    context: ExecutionContext
  ): ExecutionResult {
    // Evaluate left operand
    const leftResult = this.evaluateOperand(node.left, context);
    if (leftResult.error) {
      return leftResult;
    }

    // Evaluate right operand
    const rightResult = this.evaluateOperand(node.right, context);
    if (rightResult.error) {
      return rightResult;
    }

    const left = leftResult.value!;
    const right = rightResult.value!;

    // Perform the operation
    let result: number;

    switch (node.operator) {
      case '+':
        result = this.add(left, right);
        break;
      case '-':
        result = this.subtract(left, right);
        break;
      case '*':
        result = this.multiply(left, right);
        break;
      case '/':
        result = this.divide(left, right);
        break;
      case '%':
        result = this.modulo(left, right);
        break;
      default:
        return {
          error: new Error(`Unknown arithmetic operator: ${node.operator}`)
        };
    }

    return { value: createValue(result) };
  }

  /**
   * Execute a literal expression
   */
  private executeLiteralExpression(
    node: LiteralExpression,
    context: ExecutionContext
  ): ExecutionResult {
    return { value: node.value };
  }

  /**
   * Evaluate an operand (could be literal or identifier)
   */
  private evaluateOperand(
    node: any,
    context: ExecutionContext
  ): ExecutionResult {
    if (node.type === 'LiteralExpression') {
      return { value: node.value };
    }

    if (node.type === 'IdentifierExpression') {
      // Look up variable in scope
      const scope = context.scope;
      let current: typeof scope | null = scope;
      while (current !== null) {
        if (current.variables.has(node.name)) {
          const value = current.variables.get(node.name);
          if (typeof value === 'object' && value !== null && 'type' in value) {
            return { value: value as Value };
          }
          return { value: createValue(value) };
        }
        current = current.parent;
      }
      return {
        error: new Error(`Undefined variable: '${node.name}'`)
      };
    }

    if (node.type === 'BinaryExpression') {
      // Recursively evaluate nested binary expressions
      return this.executeBinaryExpression(node, context);
    }

    return {
      error: new Error(`Cannot evaluate operand of type: ${node.type}`)
    };
  }

  /**
   * Addition
   */
  private add(left: Value, right: Value): number {
    this.ensureNumbers(left, right, '+');
    return left.data + right.data;
  }

  /**
   * Subtraction
   */
  private subtract(left: Value, right: Value): number {
    this.ensureNumbers(left, right, '-');
    return left.data - right.data;
  }

  /**
   * Multiplication
   */
  private multiply(left: Value, right: Value): number {
    this.ensureNumbers(left, right, '*');
    return left.data * right.data;
  }

  /**
   * Division
   */
  private divide(left: Value, right: Value): number {
    this.ensureNumbers(left, right, '/');

    if (right.data === 0) {
      throw new Error('Division by zero');
    }

    return left.data / right.data;
  }

  /**
   * Modulo
   */
  private modulo(left: Value, right: Value): number {
    this.ensureNumbers(left, right, '%');

    if (right.data === 0) {
      throw new Error('Modulo by zero');
    }

    return left.data % right.data;
  }

  /**
   * Ensure both operands are numbers
   */
  private ensureNumbers(left: Value, right: Value, operator: string): void {
    if (left.type !== ValueType.NUMBER) {
      throw new Error(
        `Left operand of '${operator}' must be a number, got ${left.type}`
      );
    }
    if (right.type !== ValueType.NUMBER) {
      throw new Error(
        `Right operand of '${operator}' must be a number, got ${right.type}`
      );
    }
  }
}