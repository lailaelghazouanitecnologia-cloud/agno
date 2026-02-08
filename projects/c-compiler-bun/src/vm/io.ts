/**
 * IOVM - Handles printf and scanf I/O operations
 * 
 * Processes printf and scanf function calls for input/output.
 */

import { MicroVM, ASTNode, ExecutionContext, RuntimeValue, Type } from "../core/interfaces.js";

export class IOVM implements MicroVM {
  readonly name = "IOVM";

  private handledNodeTypes = new Set([
    "FunctionCall",
  ]);

  private inputBuffer: string[] = [];
  private outputBuffer: string[] = [];

  /**
   * Execute an I/O operation
   */
  execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    if (node.type !== "FunctionCall") {
      throw new Error(`IOVM cannot handle node type: ${node.type}`);
    }

    const { functionName, arguments: args } = node as any;

    if (functionName === "printf") {
      return this.executePrintf(args, context);
    }

    if (functionName === "scanf") {
      return this.executeScanf(args, context);
    }

    throw new Error(`IOVM cannot handle function: ${functionName}`);
  }

  /**
   * Execute printf
   */
  private executePrintf(args: ASTNode[], context: ExecutionContext): void {
    if (args.length === 0) {
      throw new Error("printf requires at least one argument");
    }

    // Get format string
    const formatArg = this.resolveValue(args[0], context);
    const format = this.toString(formatArg.value);

    // Process format string and arguments
    let output = "";
    let argIndex = 1;

    for (let i = 0; i < format.length; i++) {
      if (format[i] === "%" && i + 1 < format.length) {
        const specifier = format[i + 1];
        i++; // Skip the %

        switch (specifier) {
          case "d":
          case "i":
            if (argIndex < args.length) {
              const arg = this.resolveValue(args[argIndex], context);
              output += this.toNumber(arg.value);
              argIndex++;
            }
            break;
          case "f":
            if (argIndex < args.length) {
              const arg = this.resolveValue(args[argIndex], context);
              output += this.toFloat(arg.value).toFixed(6);
              argIndex++;
            }
            break;
          case "c":
            if (argIndex < args.length) {
              const arg = this.resolveValue(args[argIndex], context);
              output += String.fromCharCode(this.toNumber(arg.value));
              argIndex++;
            }
            break;
          case "s":
            if (argIndex < args.length) {
              const arg = this.resolveValue(args[argIndex], context);
              output += this.toString(arg.value);
              argIndex++;
            }
            break;
          case "%":
            output += "%";
            break;
          default:
            output += "%" + specifier;
        }
      } else {
        output += format[i];
      }
    }

    // Add to context output
    context.output.push(output);
    this.outputBuffer.push(output);
  }

  /**
   * Execute scanf
   */
  private executeScanf(args: ASTNode[], context: ExecutionContext): void {
    if (args.length === 0) {
      throw new Error("scanf requires at least one argument");
    }

    // Get format string
    const formatArg = this.resolveValue(args[0], context);
    const format = this.toString(formatArg.value);

    // Get input from buffer
    const input = this.inputBuffer.shift() || "";
    let inputIndex = 0;
    let argIndex = 1;

    for (let i = 0; i < format.length; i++) {
      if (format[i] === "%" && i + 1 < format.length) {
        const specifier = format[i + 1];
        i++; // Skip the %

        if (argIndex < args.length) {
          const arg = args[argIndex];
          
          // Parse input value
          let value: any;
          
          switch (specifier) {
            case "d":
            case "i":
              const numMatch = input.substring(inputIndex).match(/^-?\d+/);
              if (numMatch) {
                value = parseInt(numMatch[0]);
                inputIndex += numMatch[0].length;
              }
              break;
            case "f":
              const floatMatch = input.substring(inputIndex).match(/^-?\d+\.?\d*/);
              if (floatMatch) {
                value = parseFloat(floatMatch[0]);
                inputIndex += floatMatch[0].length;
              }
              break;
            case "c":
              if (inputIndex < input.length) {
                value = input.charCodeAt(inputIndex);
                inputIndex++;
              }
              break;
            case "s":
              const strMatch = input.substring(inputIndex).match(/^\S+/);
              if (strMatch) {
                value = strMatch[0];
                inputIndex += strMatch[0].length;
              }
              break;
          }

          // Store the value (this would need to work with MemoryVM)
          if (value !== undefined) {
            this.storeValue(arg, value, context);
          }
          
          argIndex++;
        }
      } else if (format[i] === " " || format[i] === "\t" || format[i] === "\n") {
        // Skip whitespace in input
        while (inputIndex < input.length && /\s/.test(input[inputIndex])) {
          inputIndex++;
        }
      } else if (inputIndex < input.length && input[inputIndex] === format[i]) {
        inputIndex++;
      }
    }
  }

  // ============================================================================
  // Utility Methods
  // ============================================================================

  private resolveValue(node: ASTNode, context: ExecutionContext): RuntimeValue {
    if (node.type === "Number") {
      return {
        type: (node as any).dataType,
        value: (node as any).value,
      };
    }
    
    if (node.type === "String") {
      return {
        type: Type.CHAR,
        value: (node as any).value,
      };
    }
    
    return {
      type: Type.INT,
      value: 0,
    };
  }

  private storeValue(node: ASTNode, value: any, context: ExecutionContext): void {
    // In the full implementation, this would work with MemoryVM
    // to store the value at the appropriate location
    if (node.type === "Identifier") {
      // Would delegate to MemoryVM
    }
  }

  private toString(value: any): string {
    if (typeof value === "string") {
      return value;
    }
    return String(value);
  }

  private toNumber(value: any): number {
    if (typeof value === "number") {
      return value;
    }
    if (typeof value === "string") {
      return parseFloat(value);
    }
    return 0;
  }

  private toFloat(value: any): number {
    return this.toNumber(value);
  }

  /**
   * Check if this VM can handle the given node type
   */
  canHandle(nodeType: string): boolean {
    return this.handledNodeTypes.has(nodeType);
  }

  /**
   * Reset the VM state
   */
  reset(): void {
    this.inputBuffer = [];
    this.outputBuffer = [];
  }

  /**
   * Set input for scanf
   */
  setInput(input: string): void {
    this.inputBuffer = input.split("\n");
  }

  /**
   * Get output from printf
   */
  getOutput(): string[] {
    return [...this.outputBuffer];
  }

  /**
   * Clear output buffer
   */
  clearOutput(): void {
    this.outputBuffer = [];
  }
}