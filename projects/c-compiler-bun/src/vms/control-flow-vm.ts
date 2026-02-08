/**
 * ControlFlowVM
 * Handles control flow: if/else, while, for loops
 */

import { BaseMicroVM } from './interfaces.js';
import { ASTNode, NodeType, ExecutionContext, RuntimeValue, ValueType, TokenType } from './types.js';

export class ControlFlowVM extends BaseMicroVM {
  readonly name = 'ControlFlowVM';

  canHandle(node: ASTNode): boolean {
    return (
      node.type === NodeType.IF_STMT ||
      node.type === NodeType.WHILE_STMT ||
      node.type === NodeType.FOR_STMT
    );
  }

  execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    switch (node.type) {
      case NodeType.IF_STMT:
        return this.executeIf(node as any, context);
      
      case NodeType.WHILE_STMT:
        return this.executeWhile(node as any, context);
      
      case NodeType.FOR_STMT:
        return this.executeFor(node as any, context);
      
      default:
        throw new Error(`ControlFlowVM cannot handle node type: ${node.type}`);
    }
  }

  /**
   * Execute an if statement
   */
  private executeIf(node: any, context: ExecutionContext): void {
    // Evaluate condition
    const conditionValue = this.evaluateExpression(node.condition, context);
    const condition = this.toBoolean(conditionValue);

    if (condition) {
      // Execute then branch
      this.executeStatement(node.thenBranch, context);
    } else if (node.elseBranch) {
      // Execute else branch
      this.executeStatement(node.elseBranch, context);
    }
  }

  /**
   * Execute a while loop
   */
  private executeWhile(node: any, context: ExecutionContext): void {
    while (true) {
      // Check for break
      if (context.shouldBreak) {
        context.shouldBreak = false;
        break;
      }

      // Evaluate condition
      const conditionValue = this.evaluateExpression(node.condition, context);
      const condition = this.toBoolean(conditionValue);

      if (!condition) {
        break;
      }

      // Execute body
      this.executeStatement(node.body, context);

      // Check for continue
      if (context.shouldContinue) {
        context.shouldContinue = false;
        continue;
      }
    }
  }

  /**
   * Execute a for loop
   */
  private executeFor(node: any, context: ExecutionContext): void {
    // Execute initialization
    if (node.init) {
      if (node.init.type === NodeType.VAR_DECL) {
        // Variable declaration - would be handled by MemoryVM
        // For now, skip (simplified)
      } else if (node.init.type === NodeType.EXPR_STMT) {
        this.evaluateExpression(node.init.expression, context);
      } else {
        this.evaluateExpression(node.init, context);
      }
    }

    while (true) {
      // Check for break
      if (context.shouldBreak) {
        context.shouldBreak = false;
        break;
      }

      // Evaluate condition
      let condition = true;
      if (node.condition) {
        const conditionValue = this.evaluateExpression(node.condition, context);
        condition = this.toBoolean(conditionValue);
      }

      if (!condition) {
        break;
      }

      // Execute body
      this.executeStatement(node.body, context);

      // Check for continue
      if (context.shouldContinue) {
        context.shouldContinue = false;
      }

      // Execute update
      if (node.update) {
        this.evaluateExpression(node.update, context);
      }
    }
  }

  /**
   * Execute a statement
   */
  private executeStatement(stmt: any, context: ExecutionContext): void {
    switch (stmt.type) {
      case NodeType.COMPOUND_STMT:
        // Execute all statements in the block
        for (const s of stmt.statements) {
          this.executeStatement(s, context);
          
          // Check for return
          if (context.shouldReturn) {
            return;
          }
          
          // Check for break
          if (context.shouldBreak) {
            return;
          }
          
          // Check for continue
          if (context.shouldContinue) {
            return;
          }
        }
        break;
      
      case NodeType.IF_STMT:
      case NodeType.WHILE_STMT:
      case NodeType.FOR_STMT:
        // These would be handled by ControlFlowVM itself
        this.execute(stmt, context);
        break;
      
      case NodeType.EXPR_STMT:
        this.evaluateExpression(stmt.expression, context);
        break;
      
      case NodeType.RETURN_STMT:
        // Would be handled by FunctionVM
        context.shouldReturn = true;
        if (stmt.value) {
          context.returnValue = this.evaluateExpression(stmt.value, context);
        }
        break;
      
      case NodeType.VAR_DECL:
        // Would be handled by MemoryVM
        // For now, skip
        break;
      
      default:
        throw new Error(`Unknown statement type: ${stmt.type}`);
    }
  }

  /**
   * Evaluate an expression
   */
  private evaluateExpression(expr: any, context: ExecutionContext): RuntimeValue {
    switch (expr.type) {
      case NodeType.INT_LITERAL:
        return this.createValue(ValueType.INT, expr.value);
      
      case NodeType.FLOAT_LITERAL:
        return this.createValue(ValueType.FLOAT, expr.value);
      
      case NodeType.CHAR_LITERAL:
        return this.createValue(ValueType.CHAR, expr.value.charCodeAt(0));
      
      case NodeType.IDENTIFIER_EXPR:
        const varValue = this.getVariable(expr.name, context);
        if (!varValue) {
          throw new Error(`Undefined variable: ${expr.name}`);
        }
        return varValue;
      
      case NodeType.BINARY_EXPR:
        // This would be handled by ArithmeticVM or ComparisonVM
        // For simplicity, we'll do basic evaluation
        const left = this.evaluateExpression(expr.left, context);
        const right = this.evaluateExpression(expr.right, context);
        return this.evaluateBinaryOp(expr.operator, left, right);
      
      case NodeType.ASSIGN_EXPR:
        // Would be handled by MemoryVM
        const value = this.evaluateExpression(expr.value, context);
        if (expr.target.type === NodeType.IDENTIFIER_EXPR) {
          this.setVariable(expr.target.name, value, context);
        }
        return value;
      
      default:
        throw new Error(`Cannot evaluate expression of type: ${expr.type}`);
    }
  }

  /**
   * Evaluate a binary operation
   */
  private evaluateBinaryOp(operator: TokenType, left: RuntimeValue, right: RuntimeValue): RuntimeValue {
    const leftNum = this.toNumber(left);
    const rightNum = this.toNumber(right);

    // Check if it's a comparison operator
    const isComparison = 
      operator === TokenType.LESS ||
      operator === TokenType.GREATER ||
      operator === TokenType.LESS_EQUAL ||
      operator === TokenType.GREATER_EQUAL ||
      operator === TokenType.EQUAL ||
      operator === TokenType.NOT_EQUAL;

    if (isComparison) {
      let result: boolean;
      switch (operator) {
        case TokenType.LESS:
          result = leftNum < rightNum;
          break;
        case TokenType.GREATER:
          result = leftNum > rightNum;
          break;
        case TokenType.LESS_EQUAL:
          result = leftNum <= rightNum;
          break;
        case TokenType.GREATER_EQUAL:
          result = leftNum >= rightNum;
          break;
        case TokenType.EQUAL:
          result = leftNum === rightNum;
          break;
        case TokenType.NOT_EQUAL:
          result = leftNum !== rightNum;
          break;
        default:
          throw new Error(`Unknown operator: ${operator}`);
      }
      return this.createValue(ValueType.INT, result ? 1 : 0);
    }

    // Arithmetic operation
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
   * Convert a runtime value to a boolean
   */
  private toBoolean(value: RuntimeValue): boolean {
    const num = this.toNumber(value);
    return num !== 0;
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
      default:
        throw new Error(`Cannot convert ${value.type} to number`);
    }
  }
}