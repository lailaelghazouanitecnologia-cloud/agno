/**
 * ArithmeticVM - Handles arithmetic operations (+, -, *, /, %)
 */

import {
  MicroVM,
  ASTNode,
  ExecutionContext,
  RuntimeValue,
  ValueType,
  BinaryExprNode,
  UnaryExprNode,
  NodeType,
  NumberNode
} from "./interfaces.js";
import { TypeVM } from "./type-vm.js";

export class ArithmeticVM implements MicroVM {
  name = "ArithmeticVM";
  capabilities = ["+", "-", "*", "/", "%", "unary -"];

  private typeVM: TypeVM;

  constructor(typeVM: TypeVM) {
    this.typeVM = typeVM;
  }

  /**
   * Execute an arithmetic operation
   */
  public execute(node: ASTNode, context: ExecutionContext): RuntimeValue {
    switch (node.type) {
      case NodeType.BINARY_EXPR:
        return this.executeBinaryExpr(node as BinaryExprNode, context);
      case NodeType.UNARY_EXPR:
        return this.executeUnaryExpr(node as UnaryExprNode, context);
      case NodeType.NUMBER:
        return this.executeNumber(node as NumberNode, context);
      default:
        throw new Error(`ArithmeticVM cannot execute node type: ${node.type}`);
    }
  }

  /**
   * Execute binary expression
   */
  private executeBinaryExpr(node: BinaryExprNode, context: ExecutionContext): RuntimeValue {
    const operator = node.operator;

    // Check if this is an arithmetic operator
    if (!["+", "-", "*", "/", "%"].includes(operator)) {
      throw new Error(`ArithmeticVM does not support operator: ${operator}`);
    }

    // Evaluate left and right operands
    // Note: In a full implementation, this would delegate to the compiler
    // For now, we'll handle simple cases
    const left = this.evaluateOperand(node.left, context);
    const right = this.evaluateOperand(node.right, context);

    // Promote to common type
    const [promotedLeft, promotedRight] = this.typeVM.promoteToCommonType(left, right);

    // Perform operation
    return this.performOperation(operator, promotedLeft, promotedRight);
  }

  /**
   * Execute unary expression
   */
  private executeUnaryExpr(node: UnaryExprNode, context: ExecutionContext): RuntimeValue {
    const operator = node.operator;

    if (operator === "-") {
      const operand = this.evaluateOperand(node.operand, context);
      return this.negate(operand);
    }

    throw new Error(`ArithmeticVM does not support unary operator: ${operator}`);
  }

  /**
   * Execute number literal
   */
  private executeNumber(node: NumberNode, context: ExecutionContext): RuntimeValue {
    // Determine if int or float
    if (Number.isInteger(node.value)) {
      return {
        type: ValueType.INT,
        value: node.value
      };
    } else {
      return {
        type: ValueType.FLOAT,
        value: node.value
      };
    }
  }

  /**
   * Evaluate an operand (simplified - would use full compiler in production)
   */
  private evaluateOperand(node: ASTNode, context: ExecutionContext): RuntimeValue {
    if (node.type === NodeType.NUMBER) {
      return this.executeNumber(node as NumberNode, context);
    }

    // For other types, we'd need to delegate to the compiler
    // This is a placeholder
    throw new Error(`Cannot evaluate operand type: ${node.type}`);
  }

  /**
   * Perform arithmetic operation
   */
  private performOperation(operator: string, left: RuntimeValue, right: RuntimeValue): RuntimeValue {
    const leftValue = left.value;
    const rightValue = right.value;

    switch (operator) {
      case "+":
        return this.add(left, right);

      case "-":
        return this.subtract(left, right);

      case "*":
        return this.multiply(left, right);

      case "/":
        return this.divide(left, right);

      case "%":
        return this.modulo(left, right);

      default:
        throw new Error(`Unknown operator: ${operator}`);
    }
  }

  /**
   * Addition
   */
  private add(left: RuntimeValue, right: RuntimeValue): RuntimeValue {
    if (left.type === ValueType.FLOAT || right.type === ValueType.FLOAT) {
      return {
        type: ValueType.FLOAT,
        value: Number(left.value) + Number(right.value)
      };
    }

    return {
      type: ValueType.INT,
      value: Math.floor(Number(left.value)) + Math.floor(Number(right.value))
    };
  }

  /**
   * Subtraction
   */
  private subtract(left: RuntimeValue, right: RuntimeValue): RuntimeValue {
    if (left.type === ValueType.FLOAT || right.type === ValueType.FLOAT) {
      return {
        type: ValueType.FLOAT,
        value: Number(left.value) - Number(right.value)
      };
    }

    return {
      type: ValueType.INT,
      value: Math.floor(Number(left.value)) - Math.floor(Number(right.value))
    };
  }

  /**
   * Multiplication
   */
  private multiply(left: RuntimeValue, right: RuntimeValue): RuntimeValue {
    if (left.type === ValueType.FLOAT || right.type === ValueType.FLOAT) {
      return {
        type: ValueType.FLOAT,
        value: Number(left.value) * Number(right.value)
      };
    }

    return {
      type: ValueType.INT,
      value: Math.floor(Number(left.value)) * Math.floor(Number(right.value))
    };
  }

  /**
   * Division
   */
  private divide(left: RuntimeValue, right: RuntimeValue): RuntimeValue {
    const rightNum = Number(right.value);
    if (rightNum === 0) {
      throw new Error("Division by zero");
    }

    if (left.type === ValueType.FLOAT || right.type === ValueType.FLOAT) {
      return {
        type: ValueType.FLOAT,
        value: Number(left.value) / rightNum
      };
    }

    return {
      type: ValueType.INT,
      value: Math.floor(Number(left.value) / rightNum)
    };
  }

  /**
   * Modulo
   */
  private modulo(left: RuntimeValue, right: RuntimeValue): RuntimeValue {
    const rightNum = Math.floor(Number(right.value));
    if (rightNum === 0) {
      throw new Error("Modulo by zero");
    }

    return {
      type: ValueType.INT,
      value: Math.floor(Number(left.value)) % rightNum
    };
  }

  /**
   * Unary negation
   */
  private negate(operand: RuntimeValue): RuntimeValue {
    if (operand.type === ValueType.FLOAT) {
      return {
        type: ValueType.FLOAT,
        value: -Number(operand.value)
      };
    }

    return {
      type: ValueType.INT,
      value: -Math.floor(Number(operand.value))
    };
  }

  /**
   * Validate a node
   */
  public validate(node: ASTNode): boolean {
    return this.typeVM.validate(node);
  }
}