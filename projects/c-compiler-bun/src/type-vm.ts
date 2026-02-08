/**
 * TypeVM - Handles type checking and casting
 */

import {
  MicroVM,
  ASTNode,
  ExecutionContext,
  RuntimeValue,
  ValueType,
  TypeInfo,
  CType,
  BinaryExprNode,
  UnaryExprNode,
  NodeType,
  TypeError
} from "./interfaces.js";

export class TypeVM implements MicroVM {
  name = "TypeVM";

  /**
   * Execute type-related operations
   */
  public execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    // TypeVM primarily validates and doesn't produce runtime values
    return undefined;
  }

  /**
   * Validate a node's types
   */
  public validate(node: ASTNode): boolean {
    try {
      switch (node.type) {
        case NodeType.BINARY_EXPR:
          return this.validateBinaryExpr(node as BinaryExprNode);
        case NodeType.UNARY_EXPR:
          return this.validateUnaryExpr(node as UnaryExprNode);
        default:
          return true;
      }
    } catch (error) {
      return false;
    }
  }

  /**
   * Validate binary expression
   */
  private validateBinaryExpr(node: BinaryExprNode): boolean {
    const leftType = this.inferType(node.left);
    const rightType = this.inferType(node.right);

    // Check if types are compatible for the operator
    if (!this.areTypesCompatible(leftType, rightType, node.operator)) {
      throw new TypeError(
        `Type mismatch: cannot apply '${node.operator}' to ${leftType} and ${rightType}`,
        node.line
      );
    }

    return true;
  }

  /**
   * Validate unary expression
   */
  private validateUnaryExpr(node: UnaryExprNode): boolean {
    const operandType = this.inferType(node.operand);

    // Check if operator is valid for operand type
    if (node.operator === "-" && !this.isNumericType(operandType)) {
      throw new TypeError(
        `Cannot apply unary '-' to non-numeric type ${operandType}`,
        node.line
      );
    }

    if (node.operator === "&" && !this.isLValue(node.operand)) {
      throw new TypeError(
        `Cannot take address of r-value`,
        node.line
      );
    }

    return true;
  }

  /**
   * Infer the type of an expression node
   */
  public inferType(node: ASTNode): CType {
    switch (node.type) {
      case NodeType.NUMBER:
        // Check if it has decimal point
        const numNode = node as any;
        if (Number.isInteger(numNode.value)) {
          return CType.INT;
        } else {
          return CType.FLOAT;
        }

      case NodeType.STRING:
        return CType.CHAR; // Strings are char arrays

      case NodeType.IDENTIFIER:
        // Would need to look up in symbol table
        return CType.INT; // Default

      case NodeType.BINARY_EXPR:
        const binary = node as BinaryExprNode;
        const leftType = this.inferType(binary.left);
        const rightType = this.inferType(binary.right);
        return this.getBinaryResultType(leftType, rightType, binary.operator);

      case NodeType.UNARY_EXPR:
        const unary = node as UnaryExprNode;
        return this.inferType(unary.operand);

      case NodeType.CALL_EXPR:
        // Would need to look up function return type
        return CType.INT; // Default

      case NodeType.ARRAY_ACCESS:
        return CType.INT; // Default

      default:
        return CType.VOID;
    }
  }

  /**
   * Get the result type of a binary operation
   */
  private getBinaryResultType(left: CType, right: CType, operator: string): CType {
    // Comparison operators always return int
    if (["<", ">", "<=", ">=", "==", "!="].includes(operator)) {
      return CType.INT;
    }

    // For arithmetic, use type promotion
    if (left === CType.FLOAT || right === CType.FLOAT) {
      return CType.FLOAT;
    }

    return CType.INT;
  }

  /**
   * Check if two types are compatible for an operator
   */
  private areTypesCompatible(left: CType, right: CType, operator: string): boolean {
    // Comparison operators work with any numeric types
    if (["<", ">", "<=", ">=", "==", "!="].includes(operator)) {
      return this.isNumericType(left) && this.isNumericType(right);
    }

    // Arithmetic operators require numeric types
    if (["+", "-", "*", "/", "%"].includes(operator)) {
      return this.isNumericType(left) && this.isNumericType(right);
    }

    return false;
  }

  /**
   * Check if a type is numeric
   */
  private isNumericType(type: CType): boolean {
    return type === CType.INT || type === CType.FLOAT || type === CType.CHAR;
  }

  /**
   * Check if a node is an l-value (can be assigned to)
   */
  private isLValue(node: ASTNode): boolean {
    return node.type === NodeType.IDENTIFIER || node.type === NodeType.ARRAY_ACCESS;
  }

  /**
   * Cast a runtime value to a different type
   */
  public castValue(value: RuntimeValue, targetType: CType): RuntimeValue {
    switch (targetType) {
      case CType.INT:
        return {
          type: ValueType.INT,
          value: this.toInt(value)
        };

      case CType.FLOAT:
        return {
          type: ValueType.FLOAT,
          value: this.toFloat(value)
        };

      case CType.CHAR:
        return {
          type: ValueType.CHAR,
          value: String.fromCharCode(this.toInt(value))
        };

      default:
        return value;
    }
  }

  /**
   * Convert value to int
   */
  private toInt(value: RuntimeValue): number {
    switch (value.type) {
      case ValueType.INT:
        return value.value;
      case ValueType.FLOAT:
        return Math.floor(value.value);
      case ValueType.CHAR:
        return value.value.charCodeAt(0);
      case ValueType.STRING:
        return parseInt(value.value) || 0;
      default:
        return 0;
    }
  }

  /**
   * Convert value to float
   */
  private toFloat(value: RuntimeValue): number {
    switch (value.type) {
      case ValueType.INT:
      case ValueType.FLOAT:
        return Number(value.value);
      case ValueType.CHAR:
        return value.value.charCodeAt(0);
      case ValueType.STRING:
        return parseFloat(value.value) || 0.0;
      default:
        return 0.0;
    }
  }

  /**
   * Get type info for a runtime value
   */
  public getTypeInfo(value: RuntimeValue): TypeInfo {
    switch (value.type) {
      case ValueType.INT:
        return { type: CType.INT };
      case ValueType.FLOAT:
        return { type: CType.FLOAT };
      case ValueType.CHAR:
        return { type: CType.CHAR };
      case ValueType.STRING:
        return { type: CType.CHAR, isArray: true };
      case ValueType.ARRAY:
        return { type: CType.ARRAY, isArray: true };
      case ValueType.POINTER:
        return { type: CType.POINTER, isPointer: true };
      default:
        return { type: CType.VOID };
    }
  }

  /**
   * Check if two runtime values have the same type
   */
  public isSameType(a: RuntimeValue, b: RuntimeValue): boolean {
    return a.type === b.type;
  }

  /**
   * Promote values to common type for binary operations
   */
  public promoteToCommonType(a: RuntimeValue, b: RuntimeValue): [RuntimeValue, RuntimeValue] {
    // If either is float, promote both to float
    if (a.type === ValueType.FLOAT || b.type === ValueType.FLOAT) {
      return [
        this.castValue(a, CType.FLOAT),
        this.castValue(b, CType.FLOAT)
      ];
    }

    // Otherwise, both are int
    return [
      this.castValue(a, CType.INT),
      this.castValue(b, CType.INT)
    ];
  }
}