/**
 * IOVM - Handles printf and scanf I/O operations
 */

import {
  MicroVM,
  ASTNode,
  ExecutionContext,
  RuntimeValue,
  ValueType,
  CallExprNode,
  NodeType,
  StringNode,
  NumberNode,
  IdentifierNode
} from "./interfaces.js";
import { MemoryVM } from "./memory-vm.js";

export class IOVM implements MicroVM {
  name = "IOVM";
  capabilities = ["printf", "scanf"];

  private memory: MemoryVM;
  private outputBuffer: string[] = [];

  constructor(memory: MemoryVM) {
    this.memory = memory;
  }

  /**
   * Execute an I/O operation
   */
  public execute(node: ASTNode, context: ExecutionContext): RuntimeValue | void {
    if (node.type !== NodeType.CALL_EXPR) {
      throw new Error(`IOVM expects CallExprNode, got: ${node.type}`);
    }

    const callNode = node as CallExprNode;
    const callee = callNode.callee;

    switch (callee) {
      case "printf":
        return this.executePrintf(callNode, context);
      case "scanf":
        return this.executeScanf(callNode, context);
      default:
        throw new Error(`IOVM does not support function: ${callee}`);
    }
  }

  /**
   * Execute printf
   */
  private executePrintf(node: CallExprNode, context: ExecutionContext): RuntimeValue {
    if (node.args.length === 0) {
      throw new Error("printf requires at least one argument");
    }

    // Get format string
    const formatArg = node.args[0];
    let formatString = "";

    if (formatArg.type === NodeType.STRING) {
      formatString = (formatArg as StringNode).value;
    } else {
      throw new Error("printf first argument must be a string");
    }

    // Process format string and arguments
    let output = "";
    let argIndex = 1;

    for (let i = 0; i < formatString.length; i++) {
      const char = formatString[i];

      if (char === "%" && i + 1 < formatString.length) {
        const specifier = formatString[i + 1];
        i++; // Skip specifier

        switch (specifier) {
          case "d":
          case "i":
            // Integer
            if (argIndex < node.args.length) {
              const value = this.evaluateArgument(node.args[argIndex], context);
              output += this.formatInt(value);
              argIndex++;
            } else {
              output += "%d";
            }
            break;

          case "f":
            // Float
            if (argIndex < node.args.length) {
              const value = this.evaluateArgument(node.args[argIndex], context);
              output += this.formatFloat(value);
              argIndex++;
            } else {
              output += "%f";
            }
            break;

          case "c":
            // Character
            if (argIndex < node.args.length) {
              const value = this.evaluateArgument(node.args[argIndex], context);
              output += this.formatChar(value);
              argIndex++;
            } else {
              output += "%c";
            }
            break;

          case "s":
            // String
            if (argIndex < node.args.length) {
              const value = this.evaluateArgument(node.args[argIndex], context);
              output += this.formatString(value);
              argIndex++;
            } else {
              output += "%s";
            }
            break;

          case "%":
            // Literal %
            output += "%";
            break;

          default:
            // Unknown specifier, just output it
            output += "%" + specifier;
            break;
        }
      } else if (char === "\\") {
        // Handle escape sequences
        if (i + 1 < formatString.length) {
          const escapeChar = formatString[i + 1];
          i++; // Skip escaped character

          switch (escapeChar) {
            case "n":
              output += "\n";
              break;
            case "t":
              output += "\t";
              break;
            case "\\":
              output += "\\";
              break;
            case "\"":
              output += "\"";
              break;
            case "'":
              output += "'";
              break;
            default:
              output += "\\" + escapeChar;
              break;
          }
        } else {
          output += "\\";
        }
      } else {
        output += char;
      }
    }

    // Store output
    this.outputBuffer.push(output);

    // Print to console
    console.log(output);

    // Return number of characters printed
    return {
      type: ValueType.INT,
      value: output.length
    };
  }

  /**
   * Execute scanf (simplified - reads from command line)
   */
  private executeScanf(node: CallExprNode, context: ExecutionContext): RuntimeValue {
    if (node.args.length === 0) {
      throw new Error("scanf requires at least one argument");
    }

    // Get format string
    const formatArg = node.args[0];
    let formatString = "";

    if (formatArg.type === NodeType.STRING) {
      formatString = (formatArg.value as string);
    } else {
      throw new Error("scanf first argument must be a string");
    }

    // For simplicity, we'll use a mock input
    // In a real implementation, this would read from stdin
    const mockInput = "42"; // Default mock input

    // Process format string and store values
    let argIndex = 1;
    let inputIndex = 0;
    let valuesRead = 0;

    for (let i = 0; i < formatString.length; i++) {
      const char = formatString[i];

      if (char === "%" && i + 1 < formatString.length) {
        const specifier = formatString[i + 1];
        i++; // Skip specifier

        if (argIndex < node.args.length) {
          const arg = node.args[argIndex];

          // Get variable name (should be identifier or unary with &)
          let varName = "";
          if (arg.type === NodeType.IDENTIFIER) {
            varName = (arg as IdentifierNode).name;
          } else if (arg.type === NodeType.UNARY_EXPR) {
            const unary = arg as any;
            if (unary.operator === "&" && unary.operand.type === NodeType.IDENTIFIER) {
              varName = unary.operand.name;
            }
          }

          if (varName) {
            // Parse value from mock input
            const inputValue = this.parseInput(mockInput, specifier);

            // Store in memory
            switch (specifier) {
              case "d":
              case "i":
                this.memory.setVariable(varName, {
                  type: ValueType.INT,
                  value: inputValue
                });
                break;

              case "f":
                this.memory.setVariable(varName, {
                  type: ValueType.FLOAT,
                  value: inputValue
                });
                break;

              case "c":
                this.memory.setVariable(varName, {
                  type: ValueType.CHAR,
                  value: String.fromCharCode(inputValue)
                });
                break;

              default:
                break;
            }

            valuesRead++;
            argIndex++;
          }
        }
      }
    }

    // Return number of items read
    return {
      type: ValueType.INT,
      value: valuesRead
    };
  }

  /**
   * Evaluate an argument for printf
   */
  private evaluateArgument(arg: ASTNode, context: ExecutionContext): RuntimeValue {
    // Check if it's a literal
    if (arg.type === NodeType.NUMBER) {
      const numNode = arg as NumberNode;
      if (Number.isInteger(numNode.value)) {
        return { type: ValueType.INT, value: numNode.value };
      } else {
        return { type: ValueType.FLOAT, value: numNode.value };
      }
    }

    if (arg.type === NodeType.STRING) {
      return { type: ValueType.STRING, value: (arg as StringNode).value };
    }

    // Check if it's a variable reference
    if (arg.type === NodeType.IDENTIFIER) {
      const varName = (arg as IdentifierNode).name;
      const value = this.memory.getVariable(varName);
      if (value) {
        return value;
      }
    }

    // Default
    return { type: ValueType.INT, value: 0 };
  }

  /**
   * Format as integer
   */
  private formatInt(value: RuntimeValue): string {
    return Math.floor(Number(value.value)).toString();
  }

  /**
   * Format as float
   */
  private formatFloat(value: RuntimeValue): string {
    return Number(value.value).toFixed(6);
  }

  /**
   * Format as character
   */
  private formatChar(value: RuntimeValue): string {
    if (value.type === ValueType.CHAR) {
      return value.value;
    }
    return String.fromCharCode(Math.floor(Number(value.value)));
  }

  /**
   * Format as string
   */
  private formatString(value: RuntimeValue): string {
    if (value.type === ValueType.STRING) {
      return value.value;
    }
    return String(value.value);
  }

  /**
   * Parse input value (simplified)
   */
  private parseInput(input: string, specifier: string): number {
    switch (specifier) {
      case "d":
      case "i":
        return parseInt(input) || 0;
      case "f":
        return parseFloat(input) || 0.0;
      case "c":
        return input.charCodeAt(0) || 0;
      default:
        return 0;
    }
  }

  /**
   * Get output buffer
   */
  public getOutput(): string[] {
    return [...this.outputBuffer];
  }

  /**
   * Clear output buffer
   */
  public clearOutput(): void {
    this.outputBuffer = [];
  }

  /**
   * Validate an I/O node
   */
  public validate(node: ASTNode): boolean {
    if (node.type !== NodeType.CALL_EXPR) {
      return false;
    }

    const callNode = node as CallExprNode;
    const callee = callNode.callee;

    if (callee !== "printf" && callee !== "scanf") {
      return false;
    }

    // Must have at least format string argument
    if (callNode.args.length === 0) {
      return false;
    }

    // First argument must be a string
    if (callNode.args[0].type !== NodeType.STRING) {
      return false;
    }

    return true;
  }
}