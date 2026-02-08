/**
 * Base MicroVM implementation
 * All specialized VMs extend this base class
 */

import { MicroVM, ExecutionContext, ExecutionResult, VMInstruction } from '../core/interfaces.js';

export abstract class BaseMicroVM implements MicroVM {
  abstract readonly name: string;
  protected operations: Set<string> = new Set();

  /**
   * Execute a single instruction
   */
  execute(instruction: VMInstruction, context: ExecutionContext): ExecutionResult {
    if (!this.canHandle(instruction)) {
      return {
        success: false,
        error: `${this.name} cannot handle operation: ${instruction.operation}`,
      };
    }

    return this.executeInstruction(instruction, context);
  }

  /**
   * Check if this VM can handle the given instruction
   */
  canHandle(instruction: VMInstruction): boolean {
    return this.operations.has(instruction.operation);
  }

  /**
   * Reset the VM to its initial state
   */
  reset(): void {
    // Override in subclasses if needed
  }

  /**
   * Execute the instruction (to be implemented by subclasses)
   */
  protected abstract executeInstruction(
    instruction: VMInstruction,
    context: ExecutionContext
  ): ExecutionResult;

  /**
   * Register an operation that this VM can handle
   */
  protected registerOperation(operation: string): void {
    this.operations.add(operation);
  }
}