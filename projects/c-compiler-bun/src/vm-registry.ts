/**
 * MicroVM Registry
 * Manages and dispatches to specialized VM modules
 */

import { MicroVM, VMRegistry, BaseMicroVM } from './interfaces.js';
import { ASTNode, ExecutionContext, RuntimeValue } from './types.js';

export class MicroVMRegistry implements VMRegistry {
  private vms: Map<string, MicroVM>;

  constructor() {
    this.vms = new Map();
  }

  /**
   * Register a MicroVM with the registry
   */
  public register(vm: MicroVM): void {
    this.vms.set(vm.name, vm);
  }

  /**
   * Unregister a MicroVM by name
   */
  public unregister(name: string): void {
    this.vms.delete(name);
  }

  /**
   * Get a registered VM by name
   */
  public get(name: string): MicroVM | undefined {
    return this.vms.get(name);
  }

  /**
   * Find the appropriate VM to handle a given AST node
   */
  public findVM(node: ASTNode): MicroVM | undefined {
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
  public execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    const vm = this.findVM(node);
    
    if (!vm) {
      throw new Error(`No VM found to handle node type: ${node.type}`);
    }

    // Validate if the VM supports it
    if (vm.validate) {
      vm.validate(node, context);
    }

    // Execute the node
    return vm.execute(node, context);
  }

  /**
   * Get all registered VMs
   */
  public getAll(): MicroVM[] {
    return Array.from(this.vms.values());
  }

  /**
   * Reset all VMs
   */
  public resetAll(): void {
    for (const vm of this.vms.values()) {
      if (vm.reset) {
        vm.reset();
      }
    }
  }
}