/**
 * ControlFlowVM - Handles control flow constructs (if/else, while, for)
 */

import { BaseVM } from './base-vm.js';
import { ASTNode, ExecutionContext, ExecutionResult, ValueType } from '../core/interfaces.js';
import { NodeType } from '../core/types.js';

/**
 * ControlFlowVM - handles control flow constructs
 */
export class ControlFlowVM extends BaseVM {
  readonly name = 'control-flow';

  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    try {
      switch (node.type) {
        case NodeType.IF_STMT:
          return this.executeIf(node, context);

        case NodeType.WHILE_STMT:
          return this.executeWhile(node, context);

        case NodeType.FOR_STMT:
          return this.executeFor(node, context);

        case NodeType.BREAK_STMT:
          context.breakFlag = true;
          return { success: true };

        case NodeType.CONTINUE_STMT:
          context.continueFlag = true;
          return { success: true };

        default:
          return {
            success: false,
            error: `ControlFlowVM cannot handle node type: ${node.type}`,
          };
      }
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  canHandle(node: ASTNode): boolean {
    return (
      node.type === NodeType.IF_STMT ||
      node.type === NodeType.WHILE_STMT ||
      node.type === NodeType.FOR_STMT ||
      node.type === NodeType.BREAK_STMT ||
      node.type === NodeType.CONTINUE_STMT
    );
  }

  private executeIf(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const ifStmt = node as {
      condition: ASTNode;
      thenBranch: ASTNode;
      elseBranch: ASTNode | null;
    };

    const conditionValue = this.evaluateCondition(ifStmt.condition, context);
    const condition = this.toBoolean(conditionValue);

    if (condition) {
      return this.executeStatement(ifStmt.thenBranch, context);
    } else if (ifStmt.elseBranch) {
      return this.executeStatement(ifStmt.elseBranch, context);
    }

    return { success: true };
  }

  private executeWhile(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const whileStmt = node as {
      condition: ASTNode;
      body: ASTNode;
    };

    context.inLoop = true;
    context.breakFlag = false;
    context.continueFlag = false;

    while (true) {
      // Check for break
      if (context.breakFlag) {
        context.breakFlag = false;
        break;
      }

      // Check for continue
      if (context.continueFlag) {
        context.continueFlag = false;
        continue;
      }

      // Evaluate condition
      const conditionValue = this.evaluateCondition(whileStmt.condition, context);
      const condition = this.toBoolean(conditionValue);

      if (!condition) {
        break;
      }

      // Execute body
      const result = this.executeStatement(whileStmt.body, context);

      if (!result.success) {
        return result;
      }
    }

    context.inLoop = false;
    return { success: true };
  }

  private executeFor(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const forStmt = node as {
      init: ASTNode | null;
      condition: ASTNode | null;
      update: ASTNode | null;
      body: ASTNode;
    };

    context.inLoop = true;
    context.breakFlag = false;
    context.continueFlag = false;

    // Execute initialization
    if (forStmt.init) {
      const initResult = this.executeStatement(forStmt.init, context);
      if (!initResult.success) {
        return initResult;
      }
    }

    while (true) {
      // Check for break
      if (context.breakFlag) {
        context.breakFlag = false;
        break;
      }

      // Check for continue
      if (context.continueFlag) {
        context.continueFlag = false;
        // Execute update before continuing
        if (forStmt.update) {
          const updateResult = this.executeStatement(forStmt.update, context);
          if (!updateResult.success) {
            return updateResult;
          }
        }
        continue;
      }

      // Evaluate condition
      let condition = true;
      if (forStmt.condition) {
        const conditionValue = this.evaluateCondition(forStmt.condition, context);
        condition = this.toBoolean(conditionValue);
      }

      if (!condition) {
        break;
      }

      // Execute body
      const result = this.executeStatement(forStmt.body, context);

      if (!result.success) {
        return result;
      }

      // Execute update
      if (forStmt.update) {
        const updateResult = this.executeStatement(forStmt.update, context);
        if (!updateResult.success) {
          return updateResult;
        }
      }
    }

    context.inLoop = false;
    return { success: true };
  }

  private evaluateCondition(node: ASTNode, context: ExecutionContext): ValueType {
    // This would typically delegate to the appropriate VM
    // For now, we'll handle simple cases
    if (node.type === NodeType.INTEGER_LITERAL) {
      return (node as { value: number }).value !== 0;
    }
    if (node.type === NodeType.FLOAT_LITERAL) {
      return (node as { value: number }).value !== 0;
    }
    if (node.type === NodeType.IDENTIFIER_EXPR) {
      const name = (node as { name: string }).name;
      const value = context.memory.get(name);
      if (value === undefined) {
        throw new Error(`Undefined variable: ${name}`);
      }
      return value;
    }

    // For binary expressions, we'd need to delegate to the appropriate VM
    // This is a placeholder
    throw new Error(`Cannot evaluate condition node type: ${node.type}`);
  }

  private executeStatement(node: ASTNode, context: ExecutionContext): ExecutionResult {
    // This would typically delegate to the appropriate VM
    // For now, we'll handle compound statements
    if (node.type === NodeType.COMPOUND_STMT) {
      const compound = node as { statements: ASTNode[] };
      for (const stmt of compound.statements) {
        const result = this.executeStatement(stmt, context);
        if (!result.success) {
          return result;
        }
        if (context.hasReturned) {
          break;
        }
        if (context.breakFlag || context.continueFlag) {
          break;
        }
      }
      return { success: true };
    }

    // For other statement types, we'd need to delegate
    // This is a placeholder
    return { success: true };
  }

  private toBoolean(value: ValueType): boolean {
    if (typeof value === 'boolean') {
      return value;
    }
    if (typeof value === 'number') {
      return value !== 0;
    }
    if (typeof value === 'string') {
      return value.length > 0;
    }
    if (value === null) {
      return false;
    }
    return true;
  }
}