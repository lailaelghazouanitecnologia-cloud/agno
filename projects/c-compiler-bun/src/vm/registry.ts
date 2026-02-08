import { MicroVM, ASTNode, VMContext, VMValue } from "../interfaces/index.js";

/**
 * MicroVM Registry - Central dispatcher for all specialized VMs
 */
export class MicroVMRegistry {
  private vms: Map<string, MicroVM> = new Map();
  
  /**
   * Register a MicroVM
   */
  public register(vm: MicroVM): void {
    this.vms.set(vm.name, vm);
    vm.initialize();
  }
  
  /**
   * Get a registered MicroVM by name
   */
  public get(name: string): MicroVM | undefined {
    return this.vms.get(name);
  }
  
  /**
   * Execute a node using the appropriate VM
   */
  public execute(node: ASTNode, context: VMContext): VMValue | void {
    const vmName = this.getVMNameForNode(node.type);
    const vm = this.vms.get(vmName);
    
    if (!vm) {
      throw new Error(`No VM registered for node type: ${node.type}`);
    }
    
    return vm.execute(node, context);
  }
  
  /**
   * Map node types to VM names
   */
  private getVMNameForNode(nodeType: string): string {
    // Arithmetic operations
    if (nodeType.includes("BINARY_EXPR") || nodeType.includes("UNARY_EXPR")) {
      return "ArithmeticVM";
    }
    
    // Comparison operations
    if (nodeType === "BINARY_EXPR") {
      return "ComparisonVM";
    }
    
    // Control flow
    if (nodeType.includes("IF_STMT") || 
        nodeType.includes("WHILE_STMT") || 
        nodeType.includes("FOR_STMT")) {
      return "ControlFlowVM";
    }
    
    // Functions
    if (nodeType.includes("FUNCTION") || 
        nodeType.includes("CALL_EXPR") || 
        nodeType.includes("RETURN_STMT")) {
      return "FunctionVM";
    }
    
    // Memory/Variables
    if (nodeType.includes("DECL_STMT") || 
        nodeType.includes("IDENTIFIER_EXPR") || 
        nodeType.includes("ASSIGN_EXPR") ||
        nodeType.includes("ARRAY_ACCESS_EXPR") ||
        nodeType.includes("POINTER_DEREF_EXPR") ||
        nodeType.includes("ADDRESS_OF_EXPR")) {
      return "MemoryVM";
    }
    
    // I/O
    if (nodeType.includes("CALL_EXPR")) {
      // Check if it's printf/scanf
      return "IOVM";
    }
    
    // Default to MemoryVM for most things
    return "MemoryVM";
  }
  
  /**
   * Reset all VMs
   */
  public reset(): void {
    for (const vm of this.vms.values()) {
      vm.reset();
    }
  }
  
  /**
   * Get all registered VM names
   */
  public getRegisteredVMs(): string[] {
    return Array.from(this.vms.keys());
  }
}