/**
 * TypeVM
 * Handles type checking and casting
 */

import { BaseMicroVM } from './interfaces.js';
import { ASTNode, NodeType, ExecutionContext, RuntimeValue, ValueType, TypeInfo, CType } from './types.js';

export class TypeVM extends BaseMicroVM {
  readonly name = 'TypeVM';

  // Symbol table for type information
  private symbolTable: Map<string, TypeInfo>;

  constructor() {
    super();
    this.symbolTable = new Map();
  }

  canHandle(node: ASTNode): boolean {
    return (
      node.type === NodeType.VAR_DECL ||
      node.type === NodeType.ARRAY_DECL ||
      node.type === NodeType.PARAM_DECL ||
      node.type === NodeType.FUNCTION_DECL ||
      node.type === NodeType.FUNCTION_DEF
    );
  }

  execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    switch (node.type) {
      case NodeType.VAR_DECL:
        this.registerVariable(node as any);
        break;
      
      case NodeType.ARRAY_DECL:
        this.registerArray(node as any);
        break;
      
      case NodeType.PARAM_DECL:
        this.registerParameter(node as any);
        break;
      
      case NodeType.FUNCTION_DECL:
        this.registerFunction(node as any);
        break;
      
      case NodeType.FUNCTION_DEF:
        this.registerFunction(node as any);
        break;
      
      default:
        throw new Error(`TypeVM cannot handle node type: ${node.type}`);
    }
  }

  /**
   * Register a variable in the symbol table
   */
  private registerVariable(node: any): void {
    this.symbolTable.set(node.name, node.varType);
  }

  /**
   * Register an array in the symbol table
   */
  private registerArray(node: any): void {
    const arrayType: TypeInfo = {
      base: node.elementType.base,
      pointerDepth: node.elementType.pointerDepth,
      isArray: true,
      arraySize: 0 // Size would be determined at runtime
    };
    this.symbolTable.set(node.name, arrayType);
  }

  /**
   * Register a parameter in the symbol table
   */
  private registerParameter(node: any): void {
    this.symbolTable.set(node.name, node.paramType);
  }

  /**
   * Register a function in the symbol table
   */
  private registerFunction(node: any): void {
    const functionType: TypeInfo = {
      base: node.returnType.base,
      pointerDepth: node.returnType.pointerDepth,
      isArray: false
    };
    this.symbolTable.set(node.name, functionType);
  }

  /**
   * Get type information for a symbol
   */
  public getType(symbol: string): TypeInfo | undefined {
    return this.symbolTable.get(symbol);
  }

  /**
   * Check if two types are compatible
   */
  public areTypesCompatible(type1: TypeInfo, type2: TypeInfo): boolean {
    // Same base type and pointer depth
    if (type1.base === type2.base && type1.pointerDepth === type2.pointerDepth) {
      return true;
    }

    // Int and char are compatible
    if (
      (type1.base === CType.INT && type2.base === CType.CHAR) ||
      (type1.base === CType.CHAR && type2.base === CType.INT)
    ) {
      return type1.pointerDepth === type2.pointerDepth;
    }

    // Int and float are compatible (with implicit conversion)
    if (
      (type1.base === CType.INT && type2.base === CType.FLOAT) ||
      (type1.base === CType.FLOAT && type2.base === CType.INT)
    ) {
      return type1.pointerDepth === type2.pointerDepth;
    }

    // Any pointer is compatible with void pointer
    if (
      (type1.base === CType.VOID && type1.pointerDepth > 0) ||
      (type2.base === CType.VOID && type2.pointerDepth > 0)
    ) {
      return type1.pointerDepth === type2.pointerDepth;
    }

    return false;
  }

  /**
   * Perform implicit type conversion
   */
  public implicitCast(value: RuntimeValue, targetType: TypeInfo): RuntimeValue {
    const targetValueType = this.cTypeToValueType(targetType);

    // If types match, no conversion needed
    if (value.type === targetValueType) {
      return value;
    }

    // Convert to target type
    switch (targetValueType) {
      case ValueType.INT:
        return this.createValue(ValueType.INT, this.toInt(value));
      
      case ValueType.FLOAT:
        return this.createValue(ValueType.FLOAT, this.toFloat(value));
      
      case ValueType.CHAR:
        return this.createValue(ValueType.CHAR, this.toChar(value));
      
      default:
        return value;
    }
  }

  /**
   * Perform explicit type cast
   */
  public explicitCast(value: RuntimeValue, targetType: TypeInfo): RuntimeValue {
    return this.implicitCast(value, targetType);
  }

  /**
   * Convert value to int
   */
  private toInt(value: RuntimeValue): number {
    switch (value.type) {
      case ValueType.INT:
        return value.value;
      case ValueType.FLOAT:
        return Math.trunc(value.value);
      case ValueType.CHAR:
        return value.value;
      default:
        return 0;
    }
  }

  /**
   * Convert value to float
   */
  private toFloat(value: RuntimeValue): number {
    switch (value.type) {
      case ValueType.INT:
        return Number(value.value);
      case ValueType.FLOAT:
        return value.value;
      case ValueType.CHAR:
        return value.value;
      default:
        return 0.0;
    }
  }

  /**
   * Convert value to char
   */
  private toChar(value: RuntimeValue): number {
    switch (value.type) {
      case ValueType.INT:
      case ValueType.FLOAT:
        return (value.value as number) & 0xFF;
      case ValueType.CHAR:
        return value.value;
      default:
        return 0;
    }
  }

  /**
   * Convert CType to ValueType
   */
  private cTypeToValueType(typeInfo: TypeInfo): ValueType {
    switch (typeInfo.base) {
      case CType.INT:
        return ValueType.INT;
      case CType.FLOAT:
        return ValueType.FLOAT;
      case CType.CHAR:
        return ValueType.CHAR;
      case VOID:
        return ValueType.VOID;
      default:
        return ValueType.INT;
    }
  }

  /**
   * Validate a node's types
   */
  public validate(node: ASTNode, context: ExecutionContext): void {
    switch (node.type) {
      case NodeType.BINARY_EXPR:
        this.validateBinaryExpr(node as any);
        break;
      
      case NodeType.ASSIGN_EXPR:
        this.validateAssignExpr(node as any);
        break;
      
      case NodeType.CALL_EXPR:
        this.validateCallExpr(node as any);
        break;
      
      case NodeType.RETURN_STMT:
        this.validateReturnStmt(node as any);
        break;
    }
  }

  /**
   * Validate a binary expression
   */
  private validateBinaryExpr(node: any): void {
    // Check if operator is valid for operand types
    // This is a simplified check
    const operator = node.operator;
    
    // Comparison operators always produce int
    if (
      operator === '<' ||
      operator === '>' ||
      operator === '<=' ||
      operator === '>=' ||
      operator === '==' ||
      operator === '!='
    ) {
      // OK
    }
  }

  /**
   * Validate an assignment expression
   */
  private validateAssignExpr(node: any): void {
    // Check if target type is compatible with value type
    // This is a simplified check
    if (node.target.type === NodeType.IDENTIFIER_EXPR) {
      const targetType = this.getType(node.target.name);
      if (targetType) {
        // Type checking would go here
      }
    }
  }

  /**
   * Validate a function call expression
   */
  private validateCallExpr(node: any): void {
    // Check if function exists and argument count matches
    // This is a simplified check
  }

  /**
   * Validate a return statement
   */
  private validateReturnStmt(node: any): void {
    // Check if return type matches function return type
    // This is a simplified check
  }

  /**
   * Reset the VM
   */
  public reset(): void {
    this.symbolTable.clear();
  }
}