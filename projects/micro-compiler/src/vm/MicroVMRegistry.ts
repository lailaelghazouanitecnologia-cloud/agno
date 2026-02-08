import { MicroVM, ASTNode, ExecutionContext, ExecutionResult } from '../types/MicroVM';

/**
 * Registry for managing and dispatching to micro-VMs
 */
export class MicroVMRegistry {
  private vms: Map<string, MicroVM> = new Map();

  /**
   * Register a micro-VM
   */
  public register(vm: MicroVM): void {
    this.vms.set(vm.getName(), vm);
  }

  /**
   * Unregister a micro-VM
   */
  public unregister(name: string): void {
    this.vms.delete(name);
  }

  /**
   * Get a micro-VM by name
   */
  public get(name: string): MicroVM | undefined {
    return this.vms.get(name);
  }

  /**
   * Find a micro-VM that can handle the given AST node
   */
  public findHandler(node: ASTNode): MicroVM | undefined {
    for (const vm of this.vms.values()) {
      if (vm.canHandle(node)) {
        return vm;
      }
    }
    return undefined;
  }

  /**
   * Execute an AST node using the appropriate micro-VM
   */
  public execute(node: ASTNode, context: ExecutionContext): ExecutionResult {
    const vm = this.findHandler(node);

    if (!vm) {
      return {
        error: new Error(`No micro-VM found to handle node type: ${node.type}`)
      };
    }

    return vm.execute(node, context);
  }

  /**
   * Get all registered micro-VMs
   */
  public getAll(): MicroVM[] {
    return Array.from(this.vms.values());
  }

  /**
   * Check if a micro-VM is registered
   */
  public has(name: string): boolean {
    return this.vms.has(name);
  }

  /**
   * Clear all registered micro-VMs
   */
  public clear(): void {
    this.vms.clear();
  }
}