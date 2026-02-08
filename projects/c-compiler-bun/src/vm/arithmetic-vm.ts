/**
 * ArithmeticVM - Handles arithmetic operations (+, -, *, /, %)
 */

import { BaseVM } from './base-vm.js';
import { ASTNode, ExecutionContext, ExecutionResult, ValueType } from '../core/interfaces.js';
import { NodeType, BinaryOp, UnaryOp } from '../core/types.js';

/**
 * ArithmeticVM - handles arithmetic operations
 */
export class ArithmeticVM extends BaseVM {
  readonly name = 'arithmetic';

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    try {
      let result: ValueType;

      switch (node.type) {
        case NodeType.BINARY_EXPR:
          result = this.executeBinary(node, context);
          break;

        case NodeType.UNARY_EXPR:
          result = this.executeUnary(node, context);
          break;

        case NodeType.INTEGER_LITERAL:
        case NodeType.FLOAT_LITERAL:
          result = (node as { value: number }).value;
          break;

        default:
          return {
            success: false,
            error: `ArithmeticVM cannot handle node type: ${node.type}`,
          };
      }

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
    return (
      node.type === NodeType.BINARY_EXPR ||
      node.type === NodeType.UNARY_EXPR ||
      node.type === NodeType.INTEGER_LITERAL ||
      node.type === NodeType.FLOAT_LITERAL
    );
  }

  private executeBinary(node: ASTNode, context: ExecutionContext): number {
    const expr = node as {
      operator: BinaryOp;
      left: ASTNode;
      right: ASTNode;
    };

    const leftResult = this.evaluateNode(expr.left, context);
    const rightResult = this.evaluateNode(expr.right, context);

    const left = this.toNumber(leftResult);
    const right = this.toNumber(rightResult);

    switch (expr.operator) {
      case BinaryOp.ADD:
        return left + right;
      case BinaryOp.SUB:
        return left - right;
      case BinaryOp.MUL:
        return left * right;
      case BinaryOp.DIV:
        if (right === 0) {
          throw new Error('Division by zero');
        }
        return left / right;
      case BinaryOp.MOD:
        if (right === 0) {
          throw new Error('Modulo by zero');
        }
        return left % right;
      default:
        throw new Error(`Unknown binary operator: ${expr.operator}`);
    }
  }

  private executeUnary(node: ASTNode, context: ExecutionContext): number {
    const expr = node as {
      operator: UnaryOp;
      operand: ASTNode;
    };

    const operandResult = this.evaluateNode(expr.operand, context);
    const operand = this.toNumber(operandResult);

    switch (expr.operator) {
      case UnaryOp.POS:
        return +operand;
      case UnaryOp.NEG:
        return -operand;
      case UnaryOp.PRE_INC:
      case UnaryOp.POST_INC:
        // For increment/decrement, we'd need to handle variable assignment
        // This is a simplified version
        return operand + 1;
      case UnaryOp.PRE_DEC:
      case UnaryOp.POST_DEC:
        return operand - 1;
      default:
        throw new Error(`Unknown unary operator: ${expr.operator}`);
    }
  }

  private evaluateNode(node: ASTNode, context: ExecutionContext): ValueType {
    // For now, just handle literals
    if (node.type === NodeType.INTEGER_LITERAL) {
      return (node as { value: number }).value;
    }
    if (node.type === NodeType.FLOAT_LITERAL) {
      return (node as { value: number }).value;
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