/**
 * ComparisonVM - Handles comparison operations (<, >, <=, >=, ==, !=)
 */

import { BaseVM } from './base-vm.js';
import { ASTNode, ExecutionContext, ExecutionResult, ValueType } from '../core/interfaces.js';
import { NodeType, BinaryOp } from '../core/types.js';

/**
 * ComparisonVM - handles comparison operations
 */
export class ComparisonVM extends BaseVM {
  readonly name = 'comparison';

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    try {
      if (node.type !== NodeType.BINARY_EXPR) {
        return {
          success: false,
          error: `ComparisonVM expects BINARY_EXPR, got: ${node.type}`,
        };
      }

      const expr = node as {
        operator: BinaryOp;
        left: ASTNode;
        right: ASTNode;
      };

      // Check if this is a comparison operator
      if (!this.isComparisonOperator(expr.operator)) {
        return {
          success: false,
          error: `Not a comparison operator: ${expr.operator}`,
        };
      }

      const leftResult = this.evaluateNode(expr.left, context);
      const rightResult = this.evaluateNode(expr.right, context);

      const result = this.compare(expr.operator, leftResult, rightResult);

      return {
        success: true,
        value: result,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  canHandle(node: ASTNode): boolean {
    if (node.type !== NodeType.BINARY_EXPR) {
      return false;
    }

    const expr = node as { operator: BinaryOp };
    return this.isComparisonOperator(expr.operator);
  }

  private isComparisonOperator(op: BinaryOp): boolean {
    return (
      op === BinaryOp.LT ||
      op === BinaryOp.GT ||
      op === BinaryOp.LTE ||
      op === BinaryOp.GTE ||
      op === BinaryOp.EQ ||
      op === BinaryOp.NEQ
    );
  }

  private compare(operator: BinaryOp, left: ValueType, right: ValueType): boolean {
    // Handle numeric comparisons
    if (typeof left === 'number' && typeof right === 'number') {
      switch (operator) {
        case BinaryOp.LT:
          return left < right;
        case BinaryOp.GT:
          return left > right;
        case BinaryOp.LTE:
          return left <= right;
        case BinaryOp.GTE:
          return left >= right;
        case BinaryOp.EQ:
          return left === right;
        case BinaryOp.NEQ:
          return left !== right;
      }
    }

    // Handle string comparisons
    if (typeof left === 'string' && typeof right === 'string') {
      switch (operator) {
        case BinaryOp.LT:
          return left < right;
        case BinaryOp.GT:
          return left > right;
        case BinaryOp.LTE:
          return left <= right;
        case BinaryOp.GTE:
          return left >= right;
        case BinaryOp.EQ:
          return left === right;
        case BinaryOp.NEQ:
          return left !== right;
      }
    }

    // Handle boolean comparisons
    if (typeof left === 'boolean' && typeof right === 'boolean') {
      switch (operator) {
        case BinaryOp.EQ:
          return left === right;
        case BinaryOp.NEQ:
          return left !== right;
        default:
          throw new Error('Cannot compare booleans with relational operators');
      }
    }

    // Handle null comparisons
    if (left === null || right === null) {
      switch (operator) {
        case BinaryOp.EQ:
          return left === right;
        case BinaryOp.NEQ:
          return left !== right;
        default:
          throw new Error('Cannot compare null with relational operators');
      }
    }

    // Try to convert to numbers for comparison
    const leftNum = this.toNumber(left);
    const rightNum = this.toNumber(right);

    switch (operator) {
      case BinaryOp.LT:
        return leftNum < rightNum;
      case BinaryOp.GT:
        return leftNum > rightNum;
      case BinaryOp.LTE:
        return leftNum <= rightNum;
      case BinaryOp.GTE:
        return leftNum >= rightNum;
      case BinaryOp.EQ:
        return leftNum === rightNum;
      case BinaryOp.NEQ:
        return leftNum !== rightNum;
    }
  }

  private evaluateNode(node: ASTNode, context: ExecutionContext): ValueType {
    if (node.type === NodeType.INTEGER_LITERAL) {
      return (node as { value: number }).value;
    }
    if (node.type === NodeType.FLOAT_LITERAL) {
      return (node as { value: number }).value;
    }
    if (node.type === NodeType.CHAR_LITERAL) {
      return (node as { value: string }).value;
    }
    if (node.type === NodeType.STRING_LITERAL) {
      return (node as { value: string }).value;
    }
    if (node.type === NodeType.IDENTIFIER_EXPR) {
      const name = (node as { name: string }).name;
      const value = context.memory.get(name);
      if (value === undefined) {
        throw new Error(`Undefined variable: ${name}`);
      }
      return value;
    }

    throw new Error(`Cannot evaluate node type: ${node.type}`);
  }

  private toNumber(value: ValueType): number {
    if (typeof value === 'number') {
      return value;
    }
    if (typeof value === 'string') {
      const num = parseFloat(value);
      if (isNaN(num)) {
        throw new Error(`Cannot convert string to number: ${value}`);
      }
      return num;
    }
    if (typeof value === 'boolean') {
      return value ? 1 : 0;
    }
    if (value === null) {
      return 0;
    }
    throw new Error(`Cannot convert to number: ${typeof value}`);
  }
}