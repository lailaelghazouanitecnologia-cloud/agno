/**
 * Abstract Syntax Tree node types for TypeScript
 */

export enum NodeType {
  // Program
  Program = "Program",

  // Statements
  VariableDeclaration = "VariableDeclaration",
  FunctionDeclaration = "FunctionDeclaration",
  ClassDeclaration = "ClassDeclaration",
  InterfaceDeclaration = "InterfaceDeclaration",
  TypeAliasDeclaration = "TypeAliasDeclaration",
  EnumDeclaration = "EnumDeclaration",
  ExpressionStatement = "ExpressionStatement",
  ReturnStatement = "ReturnStatement",
  IfStatement = "IfStatement",
  ForStatement = "ForStatement",
  WhileStatement = "WhileStatement",
  BlockStatement = "BlockStatement",
  EmptyStatement = "EmptyStatement",

  // Expressions
  Identifier = "Identifier",
  Literal = "Literal",
  BinaryExpression = "BinaryExpression",
  UnaryExpression = "UnaryExpression",
  AssignmentExpression = "AssignmentExpression",
  CallExpression = "CallExpression",
  MemberExpression = "MemberExpression",
  NewExpression = "NewExpression",
  ArrayExpression = "ArrayExpression",
  ObjectExpression = "ObjectExpression",
  Property = "Property",
  ArrowFunctionExpression = "ArrowFunctionExpression",
  FunctionExpression = "FunctionExpression",
  ConditionalExpression = "ConditionalExpression",
  ThisExpression = "ThisExpression",

  // Types
  TypeAnnotation = "TypeAnnotation",
  TypeReference = "TypeReference",
  UnionType = "UnionType",
  IntersectionType = "IntersectionType",
  ArrayType = "ArrayType",
  TupleType = "TupleType",
  FunctionType = "FunctionType",
  LiteralType = "LiteralType",
  StringType = "StringType",
  NumberType = "NumberType",
  BooleanType = "BooleanType",
  VoidType = "VoidType",
  AnyType = "AnyType",
  UnknownType = "UnknownType",
  NullType = "NullType",
  UndefinedType = "UndefinedType",
  GenericType = "GenericType",

  // Class members
  MethodDefinition = "MethodDefinition",
  PropertyDefinition = "PropertyDefinition",
  ConstructorDefinition = "ConstructorDefinition",

  // Parameters
  Parameter = "Parameter",
  RestParameter = "RestParameter",

  // Other
  ImportDeclaration = "ImportDeclaration",
  ExportDeclaration = "ExportDeclaration",
  TSModuleDeclaration = "TSModuleDeclaration",
}

export interface ASTNode {
  type: NodeType;
  loc?: SourceLocation;
}

export interface SourceLocation {
  start: { line: number; column: number };
  end: { line: number; column: number };
}

export interface Program extends ASTNode {
  type: NodeType.Program;
  body: Statement[];
}

export type Statement =
  | VariableDeclaration
  | FunctionDeclaration
  | ClassDeclaration
  | InterfaceDeclaration
  | TypeAliasDeclaration
  | EnumDeclaration
  | ExpressionStatement
  | ReturnStatement
  | IfStatement
  | ForStatement
  | WhileStatement
  | BlockStatement
  | ImportDeclaration
  | ExportDeclaration
  | TSModuleDeclaration;

export interface VariableDeclaration extends ASTNode {
  type: NodeType.VariableDeclaration;
  kind: "var" | "let" | "const";
  declarations: VariableDeclarator[];
}

export interface VariableDeclarator {
  id: Identifier;
  init?: Expression;
  typeAnnotation?: TypeNode;
}

export interface FunctionDeclaration extends ASTNode {
  type: NodeType.FunctionDeclaration;
  id: Identifier;
  params: Parameter[];
  returnType?: TypeNode;
  body: BlockStatement;
  isAsync: boolean;
  isGenerator: boolean;
  typeParameters?: TypeParameter[];
}

export interface ClassDeclaration extends ASTNode {
  type: NodeType.ClassDeclaration;
  id: Identifier;
  superClass?: Identifier;
  body: ClassElement[];
  implements?: TypeReference[];
  decorators?: Decorator[];
}

export type ClassElement =
  | MethodDefinition
  | PropertyDefinition
  | ConstructorDefinition;

export interface MethodDefinition extends ASTNode {
  type: NodeType.MethodDefinition;
  key: Identifier | string;
  value: FunctionExpression;
  kind: "method" | "get" | "set";
  isStatic: boolean;
  accessModifier?: "public" | "private" | "protected";
  decorators?: Decorator[];
}

export interface PropertyDefinition extends ASTNode {
  type: NodeType.PropertyDefinition;
  key: Identifier;
  value?: Expression;
  typeAnnotation?: TypeNode;
  isStatic: boolean;
  accessModifier?: "public" | "private" | "protected";
  decorators?: Decorator[];
}

export interface ConstructorDefinition extends ASTNode {
  type: NodeType.ConstructorDefinition;
  params: Parameter[];
  body: BlockStatement;
  accessModifier?: "public" | "private" | "protected";
}

export interface InterfaceDeclaration extends ASTNode {
  type: NodeType.InterfaceDeclaration;
  id: Identifier;
  extends?: TypeReference[];
  body: PropertySignature[];
  typeParameters?: TypeParameter[];
}

export interface PropertySignature {
  key: Identifier;
  typeAnnotation: TypeNode;
  isOptional: boolean;
  isReadonly: boolean;
}

export interface TypeAliasDeclaration extends ASTNode {
  type: NodeType.TypeAliasDeclaration;
  id: Identifier;
  typeAnnotation: TypeNode;
  typeParameters?: TypeParameter[];
}

export interface EnumDeclaration extends ASTNode {
  type: NodeType.EnumDeclaration;
  id: Identifier;
  members: EnumMember[];
  isConst: boolean;
}

export interface EnumMember {
  id: Identifier;
  init?: string | number;
}

export interface ExpressionStatement extends ASTNode {
  type: NodeType.ExpressionStatement;
  expression: Expression;
}

export interface ReturnStatement extends ASTNode {
  type: NodeType.ReturnStatement;
  argument?: Expression;
}

export interface IfStatement extends ASTNode {
  type: NodeType.IfStatement;
  test: Expression;
  consequent: Statement;
  alternate?: Statement;
}

export interface ForStatement extends ASTNode {
  type: NodeType.ForStatement;
  init?: VariableDeclaration | Expression;
  test?: Expression;
  update?: Expression;
  body: Statement;
}

export interface WhileStatement extends ASTNode {
  type: NodeType.WhileStatement;
  test: Expression;
  body: Statement;
}

export interface BlockStatement extends ASTNode {
  type: NodeType.BlockStatement;
  body: Statement[];
}

export type Expression =
  | Identifier
  | Literal
  | BinaryExpression
  | UnaryExpression
  | AssignmentExpression
  | CallExpression
  | MemberExpression
  | NewExpression
  | ArrayExpression
  | ObjectExpression
  | ArrowFunctionExpression
  | FunctionExpression
  | ConditionalExpression
  | ThisExpression;

export interface Identifier extends ASTNode {
  type: NodeType.Identifier;
  name: string;
  typeAnnotation?: TypeNode;
}

export interface Literal extends ASTNode {
  type: NodeType.Literal;
  value: string | number | boolean | null;
  raw?: string;
}

export interface BinaryExpression extends ASTNode {
  type: NodeType.BinaryExpression;
  operator: string;
  left: Expression;
  right: Expression;
}

export interface UnaryExpression extends ASTNode {
  type: NodeType.UnaryExpression;
  operator: string;
  argument: Expression;
  prefix: boolean;
}

export interface AssignmentExpression extends ASTNode {
  type: NodeType.AssignmentExpression;
  operator: string;
  left: Expression;
  right: Expression;
}

export interface CallExpression extends ASTNode {
  type: NodeType.CallExpression;
  callee: Expression;
  arguments: Expression[];
  typeArguments?: TypeNode[];
}

export interface MemberExpression extends ASTNode {
  type: NodeType.MemberExpression;
  object: Expression;
  property: Identifier | Expression;
  computed: boolean;
}

export interface NewExpression extends ASTNode {
  type: NodeType.NewExpression;
  callee: Expression;
  arguments: Expression[];
}

export interface ArrayExpression extends ASTNode {
  type: NodeType.ArrayExpression;
  elements: (Expression | null)[];
}

export interface ObjectExpression extends ASTNode {
  type: NodeType.ObjectExpression;
  properties: Property[];
}

export interface Property {
  key: Identifier | string;
  value: Expression;
  kind: "init" | "get" | "set";
  computed: boolean;
  shorthand: boolean;
}

export interface ArrowFunctionExpression extends ASTNode {
  type: NodeType.ArrowFunctionExpression;
  params: Parameter[];
  body: BlockStatement | Expression;
  returnType?: TypeNode;
  isAsync: boolean;
  typeParameters?: TypeParameter[];
}

export interface FunctionExpression extends ASTNode {
  type: NodeType.FunctionExpression;
  id?: Identifier;
  params: Parameter[];
  body: BlockStatement;
  returnType?: TypeNode;
  isAsync: boolean;
  isGenerator: boolean;
  typeParameters?: TypeParameter[];
}

export interface ConditionalExpression extends ASTNode {
  type: NodeType.ConditionalExpression;
  test: Expression;
  consequent: Expression;
  alternate: Expression;
}

export interface ThisExpression extends ASTNode {
  type: NodeType.ThisExpression;
}

export type TypeNode =
  | TypeAnnotation
  | TypeReference
  | UnionType
  | IntersectionType
  | ArrayType
  | TupleType
  | FunctionType
  | LiteralType
  | StringType
  | NumberType
  | BooleanType
  | VoidType
  | AnyType
  | UnknownType
  | NullType
  | UndefinedType
  | GenericType;

export interface TypeAnnotation extends ASTNode {
  type: NodeType.TypeAnnotation;
  typeAnnotation: TypeNode;
}

export interface TypeReference extends ASTNode {
  type: NodeType.TypeReference;
  typeName: string;
  typeArguments?: TypeNode[];
}

export interface UnionType extends ASTNode {
  type: NodeType.UnionType;
  types: TypeNode[];
}

export interface IntersectionType extends ASTNode {
  type: NodeType.IntersectionType;
  types: TypeNode[];
}

export interface ArrayType extends ASTNode {
  type: NodeType.ArrayType;
  elementType: TypeNode;
}

export interface TupleType extends ASTNode {
  type: NodeType.TupleType;
  elementTypes: TypeNode[];
}

export interface FunctionType extends ASTNode {
  type: NodeType.FunctionType;
  params: TypeNode[];
  returnType: TypeNode;
  typeParameters?: TypeParameter[];
}

export interface LiteralType extends ASTNode {
  type: NodeType.LiteralType;
  value: string | number | boolean;
}

export interface StringType extends ASTNode {
  type: NodeType.StringType;
}

export interface NumberType extends ASTNode {
  type: NodeType.NumberType;
}

export interface BooleanType extends ASTNode {
  type: NodeType.BooleanType;
}

export interface VoidType extends ASTNode {
  type: NodeType.VoidType;
}

export interface AnyType extends ASTNode {
  type: NodeType.AnyType;
}

export interface UnknownType extends ASTNode {
  type: NodeType.UnknownType;
}

export interface NullType extends ASTNode {
  type: NodeType.NullType;
}

export interface UndefinedType extends ASTNode {
  type: NodeType.UndefinedType;
}

export interface GenericType extends ASTNode {
  type: NodeType.GenericType;
  name: string;
  typeParameters?: TypeParameter[];
}

export interface TypeParameter {
  name: string;
  constraint?: TypeNode;
  default?: TypeNode;
}

export interface Parameter {
  id: Identifier;
  typeAnnotation?: TypeNode;
  initializer?: Expression;
  isRest: boolean;
  isOptional: boolean;
  accessModifier?: "public" | "private" | "protected" | "readonly";
}

export interface ImportDeclaration extends ASTNode {
  type: NodeType.ImportDeclaration;
  source: string;
  specifiers: ImportSpecifier[];
}

export interface ImportSpecifier {
  kind: "default" | "named" | "namespace";
  local: Identifier;
  imported?: Identifier;
}

export interface ExportDeclaration extends ASTNode {
  type: NodeType.ExportDeclaration;
  declaration?: Statement;
  specifiers?: ExportSpecifier[];
  source?: string;
}

export interface ExportSpecifier {
  local: Identifier;
  exported: Identifier;
}

export interface TSModuleDeclaration extends ASTNode {
  type: NodeType.TSModuleDeclaration;
  id: Identifier | string;
  body: BlockStatement;
  isNamespace: boolean;
}

export interface Decorator {
  expression: Expression;
}