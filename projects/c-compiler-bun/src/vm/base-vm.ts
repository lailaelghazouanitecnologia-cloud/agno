/**
 * Base MicroVM interface and abstract implementation
 */

import { ASTNode, ExecutionContext, ExecutionResult } from '../core/interfaces.js';

/**
 * Base MicroVM interface - all specialized VMs must implement this
 */
export interface MicroVM {
  /** Unique name for this VM */
  readonly name: string;
  
  /** Initialize the VM with optional context */
  initialize?(context?: unknown): void;
  
  /** Execute an operation and return result */
  execute(node: ASTNode, context: ExecutionContext): ExecutionResult;
  
  /** Check if this VM can handle the given node */
  canHandle(node: ASTNode): boolean;
  
  /** Reset VM state */
  reset?(): void;
  
  /** Get current VM state */
  getState?(): unknown;
  
  /** Set VM state */
  setState?(state: unknown): void;
}

/**
 * Abstract base class for MicroVM implementations
 */
export abstract class BaseVM implements MicroVM {
  abstract readonly name: string;
  
  protected state: Map<string, unknown> = new Map();

  initialize?(context?: unknown): void {
    // Override in subclasses if needed
  }

  abstract execute(node: ASTNode, context: ExecutionContext): ExecutionResult;
  abstract canHandle(node: ASTNode): boolean;

  reset?(): void {
    this.state.clear();
  }

  getState?(): unknown {
    return Object.fromEntries(this.state);
  }

  setState?(state: unknown): void {
    this.state.clear();
    if (state && typeof state === 'object') {
      for (const [key, value] of Object.entries(state)) {
        this.state.set(key, value);
      }
    }
  }

  protected setStateValue(key: string, value: unknown): void {
    this.state.set(key, value);
  }

  protected getStateValue<T = unknown>(key: string): T | undefined {
    return this.state.get(key) as T;
  }
}