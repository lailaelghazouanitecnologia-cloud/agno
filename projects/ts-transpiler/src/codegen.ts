/**
 * Code generator - converts AST to JavaScript code
 */

import * as AST from "./ast";

export class CodeGenerator {
  private indent: number = 0;
  private indentString: string = "  ";

  public generate(program: AST.Program): string {
    return this.generateProgram(program);
  }

  private generateProgram(program: AST.Program): string {
    const statements = program.body.map(stmt => this.generateStatement(stmt));
    return statements.filter(s => s !== "").join("\n");
  }

  private generateStatement(stmt: AST.Statement): string {
    switch (stmt.type) {
      case AST.NodeType.VariableDeclaration:
        return this.generateVariableDeclaration(stmt);
      case AST.NodeType.FunctionDeclaration:
        return this.generateFunctionDeclaration(stmt);
      case AST.NodeType.ClassDeclaration:
        return this.generateClassDeclaration(stmt);
      case AST.NodeType.EnumDeclaration:
        return this.generateEnumDeclaration(stmt);
      case AST.NodeType.ExpressionStatement:
        return this.generateExpressionStatement(stmt);
      case AST.NodeType.ReturnStatement:
        return this.generateReturnStatement(stmt);
      case AST.NodeType.IfStatement:
        return this.generateIfStatement(stmt);
      case AST.NodeType.ForStatement:
        return this.generateForStatement(stmt);
      case AST.NodeType.WhileStatement:
        return this.generateWhileStatement(stmt);
      case AST.NodeType.BlockStatement:
        return this.generateBlockStatement(stmt);
      case AST.NodeType.ImportDeclaration:
        return this.generateImportDeclaration(stmt);
      case AST.NodeType.ExportDeclaration:
        return this.generateExportDeclaration(stmt);
      default:
        return "";
    }
  }

  private generateVariableDeclaration(stmt: AST.VariableDeclaration): string {
    const declarations = stmt.declarations.map(decl => {
      let result = decl.id.name;

      if (decl.init) {
        result += " = " + this.generateExpression(decl.init);
      }

      return result;
    });

    return `${stmt.kind} ${declarations.join(", ")};`;
  }

  private generateFunctionDeclaration(stmt: AST.FunctionDeclaration): string {
    let result = "";

    if (stmt.isAsync) {
      result += "async ";
    }

    result += "function " + stmt.id.name;

    result += "(" + this.generateParameters(stmt.params) + ")";

    result += " " + this.generateBlockStatement(stmt.body);

    return result;
  }

  private generateClassDeclaration(stmt: AST.ClassDeclaration): string {
    let result = "class " + stmt.id.name;

    if (stmt.superClass) {
      result += " extends " + stmt.superClass.name;
    }

    result += " {\n";
    this.indent++;

    for (const element of stmt.body) {
      result += this.generateClassElement(element) + "\n";
    }

    this.indent--;
    result += "}";

    return result;
  }

  private generateClassElement(element: AST.ClassElement): string {
    switch (element.type) {
      case AST.NodeType.MethodDefinition:
        return this.generateMethodDefinition(element);
      case AST.NodeType.PropertyDefinition:
        return this.generatePropertyDefinition(element);
      case AST.NodeType.ConstructorDefinition:
        return this.generateConstructorDefinition(element);
      default:
        return "";
    }
  }

  private generateMethodDefinition(method: AST.MethodDefinition): string {
    let result = this.getIndent();

    if (method.isStatic) {
      result += "static ";
    }

    if (method.value.isAsync) {
      result += "async ";
    }

    if (method.value.isGenerator) {
      result += "*";
    }

    result += method.key + "(";
    result += this.generateParameters(method.value.params);
    result += ") ";

    result += this.generateBlockStatement(method.value.body);

    return result;
  }

  private generatePropertyDefinition(prop: AST.PropertyDefinition): string {
    let result = this.getIndent();

    if (prop.isStatic) {
      result += "static ";
    }

    result += prop.key.name;

    if (prop.value) {
      result += " = " + this.generateExpression(prop.value);
    }

    result += ";";

    return result;
  }

  private generateConstructorDefinition(ctor: AST.ConstructorDefinition): string {
    let result = this.getIndent() + "constructor(";
    result += this.generateParameters(ctor.params);
    result += ") ";

    result += this.generateBlockStatement(ctor.body);

    return result;
  }

  private generateEnumDeclaration(stmt: AST.EnumDeclaration): string {
    let result = "var " + stmt.id.name + ";\n";
    result += "(function (" + stmt.id.name + ") {\n";
    this.indent++;

    for (let i = 0; i < stmt.members.length; i++) {
      const member = stmt.members[i];
      const indent = this.getIndent();

      if (member.init !== undefined) {
        result += indent + stmt.id.name + "[" + JSON.stringify(member.id.name) + "] = " + JSON.stringify(member.init) + ";\n";
      } else {
        result += indent + stmt.id.name + "[" + JSON.stringify(member.id.name) + "] = " + i + ";\n";
      }
    }

    this.indent--;
    result += "})(" + stmt.id.name + " || (" + stmt.id.name + " = {}));";

    return result;
  }

  private generateExpressionStatement(stmt: AST.ExpressionStatement): string {
    return this.generateExpression(stmt.expression) + ";";
  }

  private generateReturnStatement(stmt: AST.ReturnStatement): string {
    if (stmt.argument) {
      return "return " + this.generateExpression(stmt.argument) + ";";
    }
    return "return;";
  }

  private generateIfStatement(stmt: AST.IfStatement): string {
    let result = "if (" + this.generateExpression(stmt.test) + ") ";
    result += this.generateStatement(stmt.consequent);

    if (stmt.alternate) {
      result += " else " + this.generateStatement(stmt.alternate);
    }

    return result;
  }

  private generateForStatement(stmt: AST.ForStatement): string {
    let result = "for (";

    if (stmt.init) {
      if (stmt.init.type === AST.NodeType.VariableDeclaration) {
        result += this.generateVariableDeclaration(stmt.init);
      } else {
        result += this.generateExpression(stmt.init);
      }
    }

    result += "; ";

    if (stmt.test) {
      result += this.generateExpression(stmt.test);
    }

    result += "; ";

    if (stmt.update) {
      result += this.generateExpression(stmt.update);
    }

    result += ") ";

    result += this.generateStatement(stmt.body);

    return result;
  }

  private generateWhileStatement(stmt: AST.WhileStatement): string {
    return "while (" + this.generateExpression(stmt.test) + ") " +
           this.generateStatement(stmt.body);
  }

  private generateBlockStatement(stmt: AST.BlockStatement): string {
    if (stmt.body.length === 0) {
      return "{}";
    }

    let result = "{\n";
    this.indent++;

    for (const bodyStmt of stmt.body) {
      result += this.getIndent() + this.generateStatement(bodyStmt) + "\n";
    }

    this.indent--;
    result += this.getIndent() + "}";

    return result;
  }

  private generateImportDeclaration(stmt: AST.ImportDeclaration): string {
    let result = "import ";

    const defaultImport = stmt.specifiers.find(s => s.kind === "default");
    const namespaceImport = stmt.specifiers.find(s => s.kind === "namespace");
    const namedImports = stmt.specifiers.filter(s => s.kind === "named");

    if (defaultImport) {
      result += defaultImport.local.name;
    }

    if (namespaceImport) {
      if (defaultImport) result += ", ";
      result += "* as " + namespaceImport.local.name;
    }

    if (namedImports.length > 0) {
      if (defaultImport || namespaceImport) result += ", ";
      result += "{ ";

      result += namedImports.map(spec => {
        if (spec.imported && spec.imported.name !== spec.local.name) {
          return spec.imported.name + " as " + spec.local.name;
        }
        return spec.local.name;
      }).join(", ");

      result += " }";
    }

    result += ' from "' + stmt.source + '";';

    return result;
  }

  private generateExportDeclaration(stmt: AST.ExportDeclaration): string {
    let result = "export ";

    if (stmt.declaration) {
      if (stmt.declaration.type === AST.NodeType.FunctionDeclaration) {
        const funcDecl = stmt.declaration;
        if (stmt.specifiers && stmt.specifiers.length > 0) {
          // export default function
          result += "default ";
        }
        result += "function " + funcDecl.id.name + "(";
        result += this.generateParameters(funcDecl.params) + ") ";
        result += this.generateBlockStatement(funcDecl.body);
      } else if (stmt.declaration.type === AST.NodeType.ClassDeclaration) {
        const classDecl = stmt.declaration;
        if (stmt.specifiers && stmt.specifiers.length > 0) {
          result += "default ";
        }
        result += "class " + classDecl.id.name;
        if (classDecl.superClass) {
          result += " extends " + classDecl.superClass.name;
        }
        result += " {\n";
        this.indent++;
        for (const element of classDecl.body) {
          result += this.generateClassElement(element) + "\n";
        }
        this.indent--;
        result += this.getIndent() + "}";
      } else if (stmt.declaration.type === AST.NodeType.VariableDeclaration) {
        result += this.generateVariableDeclaration(stmt.declaration);
      } else if (stmt.declaration.type === AST.NodeType.ExpressionStatement) {
        result += "default " + this.generateExpression(stmt.declaration.expression) + ";";
      }
    } else if (stmt.specifiers) {
      result += "{ ";

      result += stmt.specifiers.map(spec => {
        if (spec.local.name !== spec.exported.name) {
          return spec.local.name + " as " + spec.exported.name;
        }
        return spec.local.name;
      }).join(", ");

      result += " }";

      if (stmt.source) {
        result += ' from "' + stmt.source + '"';
      }

      result += ";";
    }

    return result;
  }

  private generateExpression(expr: AST.Expression): string {
    switch (expr.type) {
      case AST.NodeType.Identifier:
        return this.generateIdentifier(expr);
      case AST.NodeType.Literal:
        return this.generateLiteral(expr);
      case AST.NodeType.BinaryExpression:
        return this.generateBinaryExpression(expr);
      case AST.NodeType.UnaryExpression:
        return this.generateUnaryExpression(expr);
      case AST.NodeType.AssignmentExpression:
        return this.generateAssignmentExpression(expr);
      case AST.NodeType.CallExpression:
        return this.generateCallExpression(expr);
      case AST.NodeType.MemberExpression:
        return this.generateMemberExpression(expr);
      case AST.NodeType.NewExpression:
        return this.generateNewExpression(expr);
      case AST.NodeType.ArrayExpression:
        return this.generateArrayExpression(expr);
      case AST.NodeType.ObjectExpression:
        return this.generateObjectExpression(expr);
      case AST.NodeType.ArrowFunctionExpression:
        return this.generateArrowFunctionExpression(expr);
      case AST.NodeType.FunctionExpression:
        return this.generateFunctionExpression(expr);
      case AST.NodeType.ConditionalExpression:
        return this.generateConditionalExpression(expr);
      case AST.NodeType.ThisExpression:
        return "this";
      default:
        return "";
    }
  }

  private generateIdentifier(ident: AST.Identifier): string {
    return ident.name;
  }

  private generateLiteral(literal: AST.Literal): string {
    if (typeof literal.value === "string") {
      return JSON.stringify(literal.value);
    }
    return String(literal.value);
  }

  private generateBinaryExpression(expr: AST.BinaryExpression): string {
    return this.generateExpression(expr.left) + " " +
           expr.operator + " " +
           this.generateExpression(expr.right);
  }

  private generateUnaryExpression(expr: AST.UnaryExpression): string {
    if (expr.prefix) {
      return expr.operator + this.generateExpression(expr.argument);
    } else {
      return this.generateExpression(expr.argument) + expr.operator;
    }
  }

  private generateAssignmentExpression(expr: AST.AssignmentExpression): string {
    return this.generateExpression(expr.left) + " " +
           expr.operator + " " +
           this.generateExpression(expr.right);
  }

  private generateCallExpression(expr: AST.CallExpression): string {
    const args = expr.arguments.map(arg => this.generateExpression(arg));
    return this.generateExpression(expr.callee) + "(" + args.join(", ") + ")";
  }

  private generateMemberExpression(expr: AST.MemberExpression): string {
    const object = this.generateExpression(expr.object);

    if (expr.computed) {
      return object + "[" + this.generateExpression(expr.property) + "]";
    } else {
      return object + "." + (expr.property as AST.Identifier).name;
    }
  }

  private generateNewExpression(expr: AST.NewExpression): string {
    const args = expr.arguments.map(arg => this.generateExpression(arg));
    return "new " + this.generateExpression(expr.callee) + "(" + args.join(", ") + ")";
  }

  private generateArrayExpression(expr: AST.ArrayExpression): string {
    const elements = expr.elements.map(el => {
      if (el === null) return "";
      return this.generateExpression(el);
    });
    return "[" + elements.join(", ") + "]";
  }

  private generateObjectExpression(expr: AST.ObjectExpression): string {
    const properties = expr.properties.map(prop => {
      if (prop.shorthand) {
        return prop.key;
      }
      return JSON.stringify(prop.key) + ": " + this.generateExpression(prop.value);
    });
    return "{" + properties.join(", ") + "}";
  }

  private generateArrowFunctionExpression(expr: AST.ArrowFunctionExpression): string {
    let result = "";

    if (expr.isAsync) {
      result += "async ";
    }

    result += "(" + this.generateParameters(expr.params) + ") => ";

    if (expr.body.type === AST.NodeType.BlockStatement) {
      result += this.generateBlockStatement(expr.body);
    } else {
      result += this.generateExpression(expr.body);
    }

    return result;
  }

  private generateFunctionExpression(expr: AST.FunctionExpression): string {
    let result = "";

    if (expr.isAsync) {
      result += "async ";
    }

    result += "function";

    if (expr.id) {
      result += " " + expr.id.name;
    }

    result += "(" + this.generateParameters(expr.params) + ") ";
    result += this.generateBlockStatement(expr.body);

    return result;
  }

  private generateConditionalExpression(expr: AST.ConditionalExpression): string {
    return this.generateExpression(expr.test) + " ? " +
           this.generateExpression(expr.consequent) + " : " +
           this.generateExpression(expr.alternate);
  }

  private generateParameters(params: AST.Parameter[]): string {
    return params.map(param => {
      let result = "";

      if (param.isRest) {
        result += "...";
      }

      result += param.id.name;

      if (param.initializer) {
        result += " = " + this.generateExpression(param.initializer);
      }

      return result;
    }).join(", ");
  }

  private getIndent(): string {
    return this.indentString.repeat(this.indent);
  }
}