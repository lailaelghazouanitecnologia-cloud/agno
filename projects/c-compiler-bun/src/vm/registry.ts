/**
 * VM Registry - Central dispatcher for all MicroVMs
 */

import { MicroVM } from './MicroVM.js';
import { ASTNode, ExecutionContext, ExecutionResult } from '../core/interfaces.js';

/**
 * VM Registry - manages and dispatches to specialized VMs
 */
export class VMRegistry {
  private vms: Map<string, MicroVM> = new Map();

  /**
   * Register a VM
   */
  register(vm: MicroVM): void {
    this.vms.set(vm.name, vm);
  }

  /**
   * Unregister a VM by name
   */
  unregister(name: string): void {
    this.vms.delete(name);
  }

  /**
   * Get a VM by name
   */
  get(name: string): MicroVM | undefined {
    return this.vms.get(name);
  }

  /**
   * Find a VM that can handle the given node
   */
  find(node: ASTNode): MicroVM | undefined {
    for (const vm of this.vms.values()) {
      if (vm.canHandle(node)) {
        return vm;
      }
    }
    return undefined;
  }

  /**
   * Execute a node using the appropriate VM
   */
  execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const vm = this.find(node);
    
    if (!vm) {
      return {
        success: false,
        error: `No VM found to handle node type: ${node.type}`,
      };
    }

    return vm.execute(node, context);
  }

  /**
   * Get all registered VMs
   */
  getAll(): MicroVM[] {
    return Array.from(this.vms.values());
  }

  /**
   * Clear all VMs
   */
  clear(): void {
    this.vms.clear();
  }

  /**
   * Check if a VM is registered
   */
  has(name: string): boolean {
    return this.vms.has(name);
  }

  /**
   * Get the number of registered VMs
   */
  size(): number {
    return this.vms.size;
  }
}