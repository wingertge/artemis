initSidebarItems({"enum":[["Accessibility",""],["AssignOp",""],["BinaryOp",""],["BlockStmtOrExpr",""],["ClassMember",""],["Decl",""],["DefaultDecl",""],["ExportSpecifier",""],["Expr",""],["ExprOrSuper",""],["ImportSpecifier",""],["JSXAttrName",""],["JSXAttrOrSpread",""],["JSXAttrValue",""],["JSXElementChild",""],["JSXElementName",""],["JSXExpr",""],["JSXObject","Used for `obj` property of `JSXMemberExpr`."],["Lit",""],["MethodKind",""],["ModuleDecl",""],["ModuleItem",""],["ObjectPatProp",""],["Pat",""],["PatOrExpr",""],["PatOrTsParamProp",""],["Program",""],["Prop",""],["PropName",""],["PropOrSpread",""],["Stmt",""],["TruePlusMinus",""],["TsEntityName",""],["TsEnumMemberId",""],["TsFnOrConstructorType",""],["TsFnParam",""],["TsKeywordTypeKind",""],["TsLit",""],["TsModuleName",""],["TsModuleRef",""],["TsNamespaceBody","`namespace A.B { }` is a namespace named `A` with another TsNamespaceDecl as its body."],["TsParamPropParam",""],["TsSignatureDecl",""],["TsThisTypeOrIdent",""],["TsType",""],["TsTypeElement",""],["TsTypeOperatorOp",""],["TsTypeQueryExpr",""],["TsUnionOrIntersectionType",""],["UnaryOp",""],["UpdateOp",""],["VarDeclKind",""],["VarDeclOrExpr",""],["VarDeclOrPat",""]],"macro":[["op","Creates a corresponding operator."]],"struct":[["ArrayLit","Array literal."],["ArrayPat",""],["ArrowExpr",""],["AssignExpr",""],["AssignPat",""],["AssignPatProp","`{key}` or `{key = value}`"],["AssignProp",""],["AwaitExpr",""],["BigInt",""],["BinExpr",""],["BlockStmt","Use when only block statements are allowed."],["Bool",""],["BreakStmt",""],["CallExpr",""],["CatchClause",""],["Class",""],["ClassDecl",""],["ClassExpr","Class expression."],["ClassMethod",""],["ClassProp",""],["ComputedPropName",""],["CondExpr",""],["Constructor",""],["ContinueStmt",""],["DebuggerStmt",""],["Decorator",""],["DefaultExportSpecifier",""],["DoWhileStmt",""],["EmptyStmt",""],["ExportAll","`export * from 'mod'`"],["ExportDecl",""],["ExportDefaultDecl",""],["ExportDefaultExpr",""],["ExprOrSpread",""],["ExprStmt",""],["FnDecl",""],["FnExpr","Function expression."],["ForInStmt",""],["ForOfStmt",""],["ForStmt",""],["Function","Common parts of function and method."],["GetterProp",""],["Ident","Ident with span."],["IfStmt",""],["ImportDecl",""],["ImportDefault","e.g. `import foo from 'mod.js'`"],["ImportSpecific","e.g. local = foo, imported = None `import { foo } from 'mod.js'` e.g. local = bar, imported = Some(foo) for `import { foo as bar } from 'mod.js'`"],["ImportStarAs","e.g. `import * as foo from 'mod.js'`."],["Invalid","Represents a invalid node."],["JSXAttr",""],["JSXClosingElement",""],["JSXClosingFragment",""],["JSXElement",""],["JSXEmptyExpr",""],["JSXExprContainer",""],["JSXFragment",""],["JSXMemberExpr",""],["JSXNamespacedName","XML-based namespace syntax:"],["JSXOpeningElement",""],["JSXOpeningFragment",""],["JSXSpreadChild",""],["JSXText",""],["KeyValuePatProp","`{key: value}`"],["KeyValueProp",""],["LabeledStmt",""],["MemberExpr",""],["MetaPropExpr",""],["MethodProp",""],["Module",""],["NamedExport","`export { foo } from 'mod'` `export { foo as bar } from 'mod'`"],["NamedExportSpecifier",""],["NamespaceExportSpecifier","`export * as foo from 'src';`"],["NewExpr",""],["Null",""],["Number",""],["ObjectLit","Object literal."],["ObjectPat",""],["OptChainExpr",""],["ParenExpr",""],["PrivateMethod",""],["PrivateName",""],["PrivateProp",""],["Regex",""],["RestPat","EsTree `RestElement`"],["ReturnStmt",""],["Script",""],["SeqExpr",""],["SetterProp",""],["SpreadElement",""],["Str",""],["Super",""],["SwitchCase",""],["SwitchStmt",""],["TaggedTpl",""],["ThisExpr",""],["ThrowStmt",""],["Tpl",""],["TplElement",""],["TryStmt",""],["TsArrayType",""],["TsAsExpr",""],["TsCallSignatureDecl",""],["TsConditionalType",""],["TsConstAssertion",""],["TsConstructSignatureDecl",""],["TsConstructorType",""],["TsEnumDecl",""],["TsEnumMember",""],["TsExportAssignment","TypeScript's own parser uses ExportAssignment for both `export default` and `export =`. But for @babel/parser, `export default` is an ExportDefaultDecl, so a TsExportAssignment is always `export =`."],["TsExprWithTypeArgs",""],["TsExternalModuleRef",""],["TsFnType",""],["TsImportEqualsDecl",""],["TsImportType",""],["TsIndexSignature",""],["TsIndexedAccessType",""],["TsInferType",""],["TsInterfaceBody",""],["TsInterfaceDecl",""],["TsIntersectionType",""],["TsKeywordType",""],["TsLitType",""],["TsMappedType",""],["TsMethodSignature",""],["TsModuleBlock",""],["TsModuleDecl",""],["TsNamespaceDecl",""],["TsNamespaceExportDecl",""],["TsNonNullExpr",""],["TsOptionalType",""],["TsParamProp",""],["TsParenthesizedType",""],["TsPropertySignature",""],["TsQualifiedName",""],["TsRestType",""],["TsThisType",""],["TsTupleType",""],["TsTypeAliasDecl",""],["TsTypeAnn",""],["TsTypeAssertion",""],["TsTypeCastExpr",""],["TsTypeLit",""],["TsTypeOperator",""],["TsTypeParam",""],["TsTypeParamDecl",""],["TsTypeParamInstantiation",""],["TsTypePredicate",""],["TsTypeQuery","`typeof` operator"],["TsTypeRef",""],["TsUnionType",""],["UnaryExpr",""],["UpdateExpr",""],["VarDecl",""],["VarDeclarator",""],["WhileStmt",""],["WithStmt",""],["YieldExpr",""]],"trait":[["IdentExt",""]]});