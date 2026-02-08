/**
 * ComparisonVM - Handles relational and equality comparison operations
 */

import {
  MicroVM,
  ASTNode,
  ExecutionContext,
  RuntimeValue,
  ValueType,
  BinaryExprNode,
  NodeType
} from "./interfaces.js";
import { TypeVM } from "./type-vm.js";

export class ComparisonVM implements MicroVM {
  name = "ComparisonVM";
  capabilities = ["<", ">", "<=", ">=", "==", "!="];

  private typeVM: TypeVM;

  constructor(typeVM: TypeVM) {
    this.typeVM = typeVM;
  }

  /**
   * Execute a comparison operation
   */
  public execute(node: ASTNode, context: ExecutionContext): RuntimeValue {
    if (node.type !== NodeType.BINARY_EXPR) {
      throw new Error(`ComparisonVM expects BinaryExprNode, got: ${node.type}`);
    }

    const binaryNode = node as BinaryExprNode;
    const operator = binaryNode.operator;

    // Check if this is a comparison operator
    if (!["<", ">", "<=", ">=", "==", "!="].includes(operator)) {
      throw new Error(`ComparisonVM does not support operator: ${operator}`);
    }

    // Evaluate left and right operands
    const left = this.evaluateOperand(binaryNode.left, context);
    const right = this.evaluateOperand(binaryNode.right, context);

    // Promote to common type for comparison
    const [promotedLeft, promotedRight] = this.typeVM.promoteToCommonType(left, right);

    // Perform comparison
    return this.performComparison(operator, promotedLeft, promotedRight);
  }

  /**
   * Evaluate an operand
   */
  private evaluateOperand(node: ASTNode, context: ExecutionContext): RuntimeValue {
    // This would delegate to the compiler in a full implementation
    // For now, handle simple cases
    if (node.type === NodeType.NUMBER) {
      const numNode = node as any;
      if (Number.isInteger(numNode.value)) {
        return { type: ValueType.INT, value: numNode.value };
      } else {
        return { type: ValueType.FLOAT, value: numNode.value };
      }
    }

    throw new Error(`Cannot evaluate operand type: ${node.type}`);
  }

  /**
   * Perform comparison operation
   */
  private performComparison(operator: string, left: RuntimeValue, right: RuntimeValue): RuntimeValue {
    const leftValue = Number(left.value);
    const rightValue = Number(right.value);

    let result: boolean;

    switch (operator) {
      case "<":
        result = leftValue < rightValue;
        break;

      case ">":
        result = leftValue > rightValue;
        break;

      case "<=":
        result = leftValue <= rightValue;
        break;

      case ">=":
        result = leftValue >= rightValue;
        break;

      case "==":
        result = this.isEqual(left, right);
        break;

      case "!=":
        result = !this.isEqual(left, right);
        break;

      default:
        throw new Error(`Unknown comparison operator: ${operator}`);
    }

    // Comparisons always return int (0 or 1 in C)
    return {
      type: ValueType.INT,
      value: result ? 1 : 0
    };
  }

  /**
   * Check equality (handles different types)
   */
  private isEqual(left: RuntimeValue, right: RuntimeValue): boolean {
    // If types are different, they're not equal
    if (left.type !== right.type) {
      // But for numeric types, we can compare values
      if (this.isNumeric(left.type) && this.isNumeric(right.type)) {
        return Number(left.value) === Number(right.value);
      }
      return false;
    }

    // Same type comparison
    switch (left.type) {
      case ValueType.INT:
      case ValueType.FLOAT:
        return Number(left.value) === Number(right.value);

      case ValueType.CHAR:
        return left.value === right.value;

      case ValueType.STRING:
        return left.value === right.value;

      case ValueType.POINTER:
        return left.value === right.value; // Compare addresses

      default:
        return false;
    }
  }

  /**
   * Check if a value type is numeric
   */
  private isNumeric(type: ValueType): boolean {
    return type === ValueType.INT || type === ValueType.FLOAT || type === ValueType.CHAR;
  }

  /**
   * Validate a comparison node
   */
  public validate(node: ASTNode): boolean {
    if (node.type !== NodeType.BINARY_EXPR) {
      return false;
    }

    const binaryNode = node as BinaryExprNode;
    const operator = binaryNode.operator;

    // Must be a comparison operator
    if (!["<", ">", "<=", ">=", "==", "!="].includes(operator)) {
      return false;
    }

    // Validate with type checker
    return this.typeVM.validate(node);
  }

  /**
   * Check if a node is a comparison expression
   */
  public isComparison(node: ASTNode): boolean {
    if (node.type !== NodeType.BINARY_EXPR) {
      return false;
    }

    const binaryNode = node as BinaryExprNode;
    return ["<", ">", "<=", ">=", "==", "!="].includes(binaryNode.operator);
  }
}