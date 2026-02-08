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
 * ComparisonVM - Handles comparison operations: <, >, ==, !=, <=, >=
 */
export class ComparisonVM implements MicroVM {
  getName(): string {
    return 'ComparisonVM';
  }

  canHandle(node: ASTNode): boolean {
    if (node.type === 'BinaryExpression') {
      const expr = node as BinaryExpression;
      return ['<', '>', '<=', '>=', '==', '!='].includes(expr.operator);
    }
    return false;
  }

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    if (node.type !== 'BinaryExpression') {
      return {
        error: new Error(`ComparisonVM cannot handle node type: ${node.type}`)
      };
    }

    return this.executeBinaryExpression(node as BinaryExpression, context);
  }

  /**
   * Execute a binary comparison expression
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

    // Perform the comparison
    let result: boolean;

    switch (node.operator) {
      case '<':
        result = this.lessThan(left, right);
        break;
      case '>':
        result = this.greaterThan(left, right);
        break;
      case '<=':
        result = this.lessThanOrEqual(left, right);
        break;
      case '>=':
        result = this.greaterThanOrEqual(left, right);
        break;
      case '==':
        result = this.equal(left, right);
        break;
      case '!=':
        result = this.notEqual(left, right);
        break;
      default:
        return {
          error: new Error(`Unknown comparison operator: ${node.operator}`)
        };
    }

    return { value: createValue(result) };
  }

  /**
   * Evaluate an operand
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
      // Check if this is an arithmetic expression
      const expr = node as BinaryExpression;
      if (['+', '-', '*', '/', '%'].includes(expr.operator)) {
        // Would need ArithmeticVM to handle this
        // For now, return an error
        return {
          error: new Error('Nested arithmetic expressions not yet supported in ComparisonVM')
        };
      }
      // Recursively evaluate nested comparison expressions
      return this.executeBinaryExpression(node, context);
    }

    return {
      error: new Error(`Cannot evaluate operand of type: ${node.type}`)
    };
  }

  /**
   * Less than comparison
   */
  private lessThan(left: Value, right: Value): boolean {
    this.ensureComparable(left, right, '<');
    return left.data < right.data;
  }

  /**
   * Greater than comparison
   */
  private greaterThan(left: Value, right: Value): boolean {
    this.ensureComparable(left, right, '>');
    return left.data > right.data;
  }

  /**
   * Less than or equal comparison
   */
  private lessThanOrEqual(left: Value, right: Value): boolean {
    this.ensureComparable(left, right, '<=');
    return left.data <= right.data;
  }

  /**
   * Greater than or equal comparison
   */
  private greaterThanOrEqual(left: Value, right: Value): boolean {
    this.ensureComparable(left, right, '>=');
    return left.data >= right.data;
  }

  /**
   * Equality comparison
   */
  private equal(left: Value, right: Value): boolean {
    // For equality, we allow different types but compare values
    return left.data === right.data;
  }

  /**
   * Inequality comparison
   */
  private notEqual(left: Value, right: Value): boolean {
    return left.data !== right.data;
  }

  /**
   * Ensure operands are comparable
   */
  private ensureComparable(left: Value, right: Value, operator: string): void {
    if (left.type !== ValueType.NUMBER && left.type !== ValueType.STRING) {
      throw new Error(
        `Left operand of '${operator}' must be a number or string, got ${left.type}`
      );
    }
    if (right.type !== ValueType.NUMBER && right.type !== ValueType.STRING) {
      throw new Error(
        `Right operand of '${operator}' must be a number or string, got ${right.type}`
      );
    }

    // For ordering comparisons, types must match
    if (left.type !== right.type) {
      throw new Error(
        `Cannot compare ${left.type} with ${right.type} using '${operator}'`
      );
    }
  }
}