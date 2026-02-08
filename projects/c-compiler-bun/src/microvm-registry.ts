/**
 * MicroVM Registry - Dispatches to specialized VMs based on node type
 */

import {
  MicroVM,
  ASTNode,
  ExecutionContext,
  RuntimeValue,
  NodeType
} from "./interfaces.js";
import { MemoryVM } from "./memory-vm.js";
import { TypeVM } from "./type-vm.js";
import { ArithmeticVM } from "./arithmetic-vm.js";
import { ComparisonVM } from "./comparison-vm.js";
import { ControlFlowVM } from "./control-flow-vm.js";
import { FunctionVM } from "./function-vm.js";
import { IOVM } from "./io-vm.js";

export class MicroVMRegistry {
  private vms: Map<string, MicroVM> = new Map();
  private nodeTypeToVM: Map<NodeType, string> = new Map();

  // VM instances
  public memory: MemoryVM;
  public type: TypeVM;
  public arithmetic: ArithmeticVM;
  public comparison: ComparisonVM;
  public controlFlow: ControlFlowVM;
  public function: FunctionVM;
  public io: IOVM;

  constructor() {
    // Initialize VMs
    this.memory = new MemoryVM();
    this.type = new TypeVM();
    this.arithmetic = new ArithmeticVM(this.type);
    this.comparison = new ComparisonVM(this.type);
    this.controlFlow = new ControlFlowVM();
    this.function = new FunctionVM(this.memory);
    this.io = new IOVM(this.memory);

    // Register VMs
    this.registerVM(this.memory);
    this.registerVM(this.type);
    this.registerVM(this.arithmetic);
    this.registerVM(this.comparison);
    this.registerVM(this.controlFlow);
    this.registerVM(this.function);
    this.registerVM(this.io);

    // Set up node type to VM mappings
    this.setupMappings();
  }

  /**
   * Register a VM
   */
  public registerVM(vm: MicroVM): void {
    this.vms.set(vm.name, vm);
  }

  /**
   * Get a VM by name
   */
  public getVM(name: string): MicroVM | undefined {
    return this.vms.get(name);
  }

  /**
   * Get all registered VMs
   */
  public getAllVMs(): MicroVM[] {
    return Array.from(this.vms.values());
  }

  /**
   * Set up node type to VM mappings
   */
  private setupMappings(): void {
    // MemoryVM handles
    this.nodeTypeToVM.set(NodeType.VAR_DECL, "MemoryVM");
    this.nodeTypeToVM.set(NodeType.ARRAY_DECL, "MemoryVM");
    this.nodeTypeToVM.set(NodeType.IDENTIFIER, "MemoryVM");
    this.nodeTypeToVM.set(NodeType.ARRAY_ACCESS, "MemoryVM");

    // ArithmeticVM handles
    this.nodeTypeToVM.set(NodeType.NUMBER, "ArithmeticVM");

    // ComparisonVM handles
    // (Binary expressions are handled by multiple VMs based on operator)

    // ControlFlowVM handles
    this.nodeTypeToVM.set(NodeType.IF_STMT, "ControlFlowVM");
    this.nodeTypeToVM.set(NodeType.WHILE_STMT, "ControlFlowVM");
    this.nodeTypeToVM.set(NodeType.FOR_STMT, "ControlFlowVM");

    // FunctionVM handles
    this.nodeTypeToVM.set(NodeType.FUNCTION_DECL, "FunctionVM");
    this.nodeTypeToVM.set(NodeType.CALL_EXPR, "FunctionVM");
    this.nodeTypeToVM.set(NodeType.RETURN_STMT, "FunctionVM");

    // IOVM handles (via CallExprNode for printf/scanf)
  }

  /**
   * Dispatch a node to the appropriate VM
   */
  public dispatch(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    const vmName = this.nodeTypeToVM.get(node.type);

    if (vmName) {
      const vm = this.vms.get(vmName);
      if (vm) {
        return vm.execute(node, context);
      }
    }

    // Handle binary expressions specially
    if (node.type === NodeType.BINARY_EXPR) {
      return this.dispatchBinaryExpr(node, context);
    }

    // Handle unary expressions specially
    if (node.type === NodeType.UNARY_EXPR) {
      return this.dispatchUnaryExpr(node, context);
    }

    // Handle call expressions specially (could be IO or function)
    if (node.type === NodeType.CALL_EXPR) {
      return this.dispatchCallExpr(node, context);
    }

    throw new Error(`No VM registered for node type: ${node.type}`);
  }

  /**
   * Dispatch binary expression based on operator
   */
  private dispatchBinaryExpr(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    const binaryNode = node as any;
    const operator = binaryNode.operator;

    // Check if it's a comparison operator
    if (["<", ">", "<=", ">=", "==", "!="].includes(operator)) {
      return this.comparison.execute(node, context);
    }

    // Otherwise, it's an arithmetic operator
    if (["+", "-", "*", "/", "%"].includes(operator)) {
      return this.arithmetic.execute(node, context);
    }

    // Assignment operator
    if (operator === "=") {
      // Handle assignment through memory VM
      return this.handleAssignment(binaryNode, context);
    }

    throw new Error(`Unknown binary operator: ${operator}`);
  }

  /**
   * Dispatch unary expression based on operator
   */
  private dispatchUnaryExpr(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    const unaryNode = node as any;
    const operator = unaryNode.operator;

    // Arithmetic unary
    if (operator === "-") {
      return this.arithmetic.execute(node, context);
    }

    // Address-of operator
    if (operator === "&") {
      // Would be handled by memory VM in full implementation
      return undefined;
    }

    // Dereference operator
    if (operator === "*") {
      // Would be handled by memory VM in full implementation
      return undefined;
    }

    throw new Error(`Unknown unary operator: ${operator}`);
  }

  /**
   * Dispatch call expression based on callee
   */
  private dispatchCallExpr(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    const callNode = node as any;
    const callee = callNode.callee;

    // Check for I/O functions
    if (callee === "printf" || callee === "scanf") {
      return this.io.execute(node, context);
    }

    // Otherwise, it's a function call
    return this.function.execute(node, context);
  }

  /**
   * Handle assignment
   */
  private handleAssignment(node: any, context: ExecutionContext): RuntimeValue | void {
    // Get the target variable name
    let varName = "";
    if (node.left.type === NodeType.IDENTIFIER) {
      varName = node.left.name;
    } else if (node.left.type === NodeType.ARRAY_ACCESS) {
      // Array assignment
      const arrayAccess = node.left;
      const arrayVar = this.memory.getVariable(arrayAccess.array);
      if (arrayVar && arrayVar.type === "array") {
        // Evaluate index and set value
        // This is simplified - full implementation would evaluate expressions
        return undefined;
      }
    }

    if (varName) {
      // Evaluate right side (simplified)
      const value = { type: "int", value: 0 };
      this.memory.setVariable(varName, value);
    }

    return undefined;
  }

  /**
   * Validate a node
   */
  public validate(node: ASTNode): boolean {
    const vmName = this.nodeTypeToVM.get(node.type);

    if (vmName) {
      const vm = this.vms.get(vmName);
      if (vm && vm.validate) {
        return vm.validate(node);
      }
    }

    return true;
  }

  /**
   * Reset all VMs
   */
  public reset(): void {
    this.memory.reset();
    this.function.reset();
    this.io.clearOutput();
  }

  /**
   * Get VM capabilities
   */
  public getCapabilities(): Map<string, string[]> {
    const capabilities = new Map<string, string[]>();

    for (const [name, vm] of this.vms) {
      if (vm.capabilities) {
        capabilities.set(name, vm.capabilities);
      }
    }

    return capabilities;
  }
}