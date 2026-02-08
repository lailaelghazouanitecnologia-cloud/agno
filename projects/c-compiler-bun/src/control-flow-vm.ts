/**
 * ControlFlowVM - Handles if/else, while, and for control flow
 */

import {
  MicroVM,
  ASTNode,
  ExecutionContext,
  RuntimeValue,
  ValueType,
  IfStmtNode,
  WhileStmtNode,
  ForStmtNode,
  NodeType,
  ExprNode
} from "./interfaces.js";

export class ControlFlowVM implements MicroVM {
  name = "ControlFlowVM";
  capabilities = ["if", "else", "while", "for", "break", "continue"];

  /**
   * Execute a control flow statement
   */
  public execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    switch (node.type) {
      case NodeType.IF_STMT:
        return this.executeIf(node as IfStmtNode, context);
      case NodeType.WHILE_STMT:
        return this.executeWhile(node as WhileStmtNode, context);
      case NodeType.FOR_STMT:
        return this.executeFor(node as ForStmtNode, context);
      default:
        throw new Error(`ControlFlowVM cannot execute node type: ${node.type}`);
    }
  }

  /**
   * Execute if/else statement
   */
  private executeIf(node: IfStmtNode, context: ExecutionContext): RuntimeValue | void {
    // Evaluate condition
    const conditionValue = this.evaluateExpression(node.condition, context);
    const condition = this.isTruthy(conditionValue);

    if (condition) {
      // Execute then branch
      return this.executeStatement(node.thenBranch, context);
    } else if (node.elseBranch) {
      // Execute else branch
      return this.executeStatement(node.elseBranch, context);
    } else if (node.elseifBranches && node.elseifBranches.length > 0) {
      // Check else-if branches
      for (const elseif of node.elseifBranches) {
        const elseifCondition = this.evaluateExpression(elseif.condition, context);
        if (this.isTruthy(elseifCondition)) {
          return this.executeStatement(elseif.body, context);
        }
      }
    }

    return undefined;
  }

  /**
   * Execute while loop
   */
  private executeWhile(node: WhileStmtNode, context: ExecutionContext): RuntimeValue | void {
    while (true) {
      // Check for break
      if (context.breakLoop) {
        context.breakLoop = false;
        break;
      }

      // Evaluate condition
      const conditionValue = this.evaluateExpression(node.condition, context);
      const condition = this.isTruthy(conditionValue);

      if (!condition) {
        break;
      }

      // Check for continue
      if (context.continueLoop) {
        context.continueLoop = false;
        continue;
      }

      // Execute body
      this.executeStatement(node.body, context);
    }

    return undefined;
  }

  /**
   * Execute for loop
   */
  private executeFor(node: ForStmtNode, context: ExecutionContext): RuntimeValue | void {
    // Execute initialization
    if (node.init) {
      this.executeStatement(node.init, context);
    }

    while (true) {
      // Check for break
      if (context.breakLoop) {
        context.breakLoop = false;
        break;
      }

      // Evaluate condition
      let condition = true;
      if (node.condition) {
        const conditionValue = this.evaluateExpression(node.condition, context);
        condition = this.isTruthy(conditionValue);
      }

      if (!condition) {
        break;
      }

      // Check for continue
      if (context.continueLoop) {
        context.continueLoop = false;
        // Execute increment before continuing
        if (node.increment) {
          this.evaluateExpression(node.increment, context);
        }
        continue;
      }

      // Execute body
      this.executeStatement(node.body, context);

      // Execute increment
      if (node.increment) {
        this.evaluateExpression(node.increment, context);
      }
    }

    return undefined;
  }

  /**
   * Execute a statement
   */
  private executeStatement(stmt: ASTNode, context: ExecutionContext): RuntimeValue | void {
    // This would delegate to the compiler in a full implementation
    // For now, return undefined
    return undefined;
  }

  /**
   * Evaluate an expression
   */
  private evaluateExpression(expr: ExprNode, context: ExecutionContext): RuntimeValue {
    // This would delegate to the compiler in a full implementation
    // For now, return a default value
    return { type: ValueType.INT, value: 0 };
  }

  /**
   * Check if a value is truthy (non-zero in C)
   */
  private isTruthy(value: RuntimeValue): boolean {
    switch (value.type) {
      case ValueType.INT:
      case ValueType.FLOAT:
      case ValueType.CHAR:
        return Number(value.value) !== 0;

      case ValueType.STRING:
        return value.value.length > 0;

      case ValueType.POINTER:
        return value.value !== null && value.value !== undefined;

      default:
        return false;
    }
  }

  /**
   * Validate a control flow node
   */
  public validate(node: ASTNode): boolean {
    switch (node.type) {
      case NodeType.IF_STMT:
        return this.validateIf(node as IfStmtNode);
      case NodeType.WHILE_STMT:
        return this.validateWhile(node as WhileStmtNode);
      case NodeType.FOR_STMT:
        return this.validateFor(node as ForStmtNode);
      default:
        return false;
    }
  }

  /**
   * Validate if statement
   */
  private validateIf(node: IfStmtNode): boolean {
    // Condition must be present
    if (!node.condition) {
      return false;
    }

    // Then branch must be present
    if (!node.thenBranch) {
      return false;
    }

    return true;
  }

  /**
   * Validate while statement
   */
  private validateWhile(node: WhileStmtNode): boolean {
    // Condition must be present
    if (!node.condition) {
      return false;
    }

    // Body must be present
    if (!node.body) {
      return false;
    }

    return true;
  }

  /**
   * Validate for statement
   */
  private validateFor(node: ForStmtNode): boolean {
    // Body must be present
    if (!node.body) {
      return false;
    }

    return true;
  }

  /**
   * Handle break statement
   */
  public handleBreak(context: ExecutionContext): void {
    context.breakLoop = true;
  }

  /**
   * Handle continue statement
   */
  public handleContinue(context: ExecutionContext): void {
    context.continueLoop = true;
  }
}