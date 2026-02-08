/**
 * TypeVM - Handles type checking and casting operations
 * 
 * Performs type checking, type inference, and type casting.
 */

import { MicroVM, ASTNode, ExecutionContext, RuntimeValue, Type, TypeInfo } from "../core/interfaces.js";

export class TypeVM implements MicroVM {
  readonly name = "TypeVM";

  private handledNodeTypes = new Set([
    "BinaryOp",
    "UnaryOp",
    "Assignment",
    "Declaration",
    "FunctionCall",
    "ReturnStatement",
  ]);

  /**
   * Execute type checking and casting
   */
  execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    // TypeVM primarily performs type checking
    // It doesn't modify values but validates types
    
    switch (node.type) {
      case "BinaryOp":
        this.checkBinaryOp(node, context);
        break;
      case "UnaryOp":
        this.checkUnaryOp(node, context);
        break;
      case "Assignment":
        this.checkAssignment(node, context);
        break;
      case "Declaration":
        this.checkDeclaration(node, context);
        break;
      case "FunctionCall":
        this.checkFunctionCall(node, context);
        break;
      case "ReturnStatement":
        this.checkReturnStatement(node, context);
        break;
      default:
        throw new Error(`TypeVM cannot handle node type: ${node.type}`);
    }
  }

  /**
   * Check types for a binary operation
   */
  private checkBinaryOp(node: any, context: ExecutionContext): void {
    const { operator, left, right } = node;

    const leftType = this.inferType(left, context);
    const rightType = this.inferType(right, context);

    // Check if types are compatible
    if (!this.areTypesCompatible(leftType, rightType)) {
      throw new Error(
        `Type mismatch in binary operation '${operator}': ` +
        `${leftType} and ${rightType} are not compatible`
      );
    }
  }

  /**
   * Check types for a unary operation
   */
  private checkUnaryOp(node: any, context: ExecutionContext): void {
    const { operator, operand } = node;

    const operandType = this.inferType(operand, context);

    // Check if operator is valid for the type
    if (operator === "-" && !this.isNumericType(operandType)) {
      throw new Error(
        `Unary operator '${operator}' cannot be applied to type ${operandType}`
      );
    }

    if (operator === "*" && !this.isPointerType(operandType)) {
      throw new Error(
        `Cannot dereference non-pointer type ${operandType}`
      );
    }

    if (operator === "&" && this.isPointerType(operandType)) {
      throw new Error(
        `Cannot take address of a pointer`
      );
    }
  }

  /**
   * Check types for an assignment
   */
  private checkAssignment(node: any, context: ExecutionContext): void {
    const { target, value } = node;

    const targetType = this.inferType(target, context);
    const valueType = this.inferType(value, context);

    // Check if types are compatible for assignment
    if (!this.canAssign(targetType, valueType)) {
      throw new Error(
        `Cannot assign type ${valueType} to type ${targetType}`
      );
    }
  }

  /**
   * Check types for a declaration
   */
  private checkDeclaration(node: any, context: ExecutionContext): void {
    const { varType, initialValue } = node;

    if (initialValue) {
      const valueType = this.inferType(initialValue, context);

      if (!this.canAssign(varType, valueType)) {
        throw new Error(
          `Cannot initialize type ${varType.base} with type ${valueType}`
        );
      }
    }
  }

  /**
   * Check types for a function call
   */
  private checkFunctionCall(node: any, context: ExecutionContext): void {
    const { functionName, arguments: args } = node;

    // In the full implementation, this would check against the function signature
    // For now, we just validate that arguments are provided correctly
    
    for (const arg of args) {
      this.inferType(arg, context);
    }
  }

  /**
   * Check types for a return statement
   */
  private checkReturnStatement(node: any, context: ExecutionContext): void {
    const { value } = node;

    if (value) {
      this.inferType(value, context);
    }
  }

  // ============================================================================
  // Type Inference and Checking
  // ============================================================================

  /**
   * Infer the type of an AST node
   */
  private inferType(node: ASTNode, context: ExecutionContext): TypeInfo {
    switch (node.type) {
      case "Number":
        const numNode = node as any;
        return {
          base: numNode.dataType,
          isPointer: false,
          isArray: false,
        };

      case "String":
        return {
          base: Type.CHAR,
          isPointer: true,
          isArray: true,
        };

      case "Identifier":
        // Would look up in symbol table
        return {
          base: Type.INT,
          isPointer: false,
          isArray: false,
        };

      case "BinaryOp":
        const binNode = node as any;
        const leftType = this.inferType(binNode.left, context);
        const rightType = this.inferType(binNode.right, context);
        return this.getBinaryOpResultType(binNode.operator, leftType, rightType);

      case "UnaryOp":
        const unaryNode = node as any;
        if (unaryNode.operator === "*") {
          return {
            base: Type.INT,
            isPointer: false,
            isArray: false,
          };
        }
        if (unaryNode.operator === "&") {
          return {
            base: Type.INT,
            isPointer: true,
            isArray: false,
          };
        }
        return this.inferType(unaryNode.operand, context);

      case "ArrayAccess":
        return {
          base: Type.INT,
          isPointer: false,
          isArray: false,
        };

      case "FunctionCall":
        // Would look up function return type
        return {
          base: Type.INT,
          isPointer: false,
          isArray: false,
        };

      default:
        return {
          base: Type.INT,
          isPointer: false,
          isArray: false,
        };
    }
  }

  /**
   * Get the result type of a binary operation
   */
  private getBinaryOpResultType(operator: string, left: TypeInfo, right: TypeInfo): TypeInfo {
    // For arithmetic operations, result is the "larger" type
    if (["+", "-", "*", "/"].includes(operator)) {
      if (left.base === Type.FLOAT || right.base === Type.FLOAT) {
        return {
          base: Type.FLOAT,
          isPointer: false,
          isArray: false,
        };
      }
      return {
        base: Type.INT,
        isPointer: false,
        isArray: false,
      };
    }

    // For comparison operations, result is always int (boolean)
    if (["<", ">", "<=", ">=", "==", "!="].includes(operator)) {
      return {
        base: Type.INT,
        isPointer: false,
        isArray: false,
      };
    }

    // For modulo, result is int
    if (operator === "%") {
      return {
        base: Type.INT,
        isPointer: false,
        isArray: false,
      };
    }

    return {
      base: Type.INT,
      isPointer: false,
      isArray: false,
    };
  }

  /**
   * Check if two types are compatible
   */
  private areTypesCompatible(type1: TypeInfo, type2: TypeInfo): boolean {
    // Same base type
    if (type1.base === type2.base) {
      return true;
    }

    // Int and float are compatible (with implicit conversion)
    if (
      (type1.base === Type.INT && type2.base === Type.FLOAT) ||
      (type1.base === Type.FLOAT && type2.base === Type.INT)
    ) {
      return true;
    }

    // Char and int are compatible
    if (
      (type1.base === Type.CHAR && type2.base === Type.INT) ||
      (type1.base === Type.INT && type2.base === Type.CHAR)
    ) {
      return true;
    }

    return false;
  }

  /**
   * Check if a value can be assigned to a target type
   */
  private canAssign(target: TypeInfo, source: TypeInfo): boolean {
    // Exact match
    if (target.base === source.base) {
      return true;
    }

    // Int can be assigned to float
    if (target.base === Type.FLOAT && source.base === Type.INT) {
      return true;
    }

    // Char can be assigned to int
    if (target.base === Type.INT && source.base === Type.CHAR) {
      return true;
    }

    // Int can be assigned to char (with truncation)
    if (target.base === Type.CHAR && source.base === Type.INT) {
      return true;
    }

    // Any pointer can be assigned to void*
    if (target.base === Type.VOID && target.isPointer && source.isPointer) {
      return true;
    }

    return false;
  }

  /**
   * Check if a type is numeric
   */
  private isNumericType(type: TypeInfo): boolean {
    return type.base === Type.INT || type.base === Type.FLOAT || type.base === Type.CHAR;
  }

  /**
   * Check if a type is a pointer
   */
  private isPointerType(type: TypeInfo): boolean {
    return type.isPointer;
  }

  /**
   * Perform type casting
   */
  private castValue(value: RuntimeValue, targetType: TypeInfo): RuntimeValue {
    if (value.type === targetType.base) {
      return value;
    }

    let result: any;

    switch (targetType.base) {
      case Type.INT:
        result = Math.trunc(this.toNumber(value.value));
        break;
      case Type.FLOAT:
        result = this.toNumber(value.value);
        break;
      case Type.CHAR:
        result = String.fromCharCode(this.toNumber(value.value) & 0xFF);
        break;
      default:
        result = value.value;
    }

    return {
      type: targetType.base,
      value: result,
    };
  }

  private toNumber(value: any): number {
    if (typeof value === "number") {
      return value;
    }
    if (typeof value === "string") {
      return parseFloat(value);
    }
    return 0;
  }

  /**
   * Check if this VM can handle the given node type
   */
  canHandle(nodeType: string): boolean {
    return this.handledNodeTypes.has(nodeType);
  }

  /**
   * Reset the VM state
   */
  reset(): void {
    // TypeVM is stateless, nothing to reset
  }
}