use oxc_ast::ast::*;

use crate::extract;
use crate::util;

pub fn find_runtime_syntax(body: &[Statement], class_name: &str) -> Option<String> {
    let _class = extract::find_class(body, class_name)?;

    let runtime_id = find_runtime_identifier(body, class_name);
    if let Some(id) = runtime_id
        && let Some(resolved) = resolve_runtime_value(body, &id, 0)
    {
        match resolved.as_str() {
            "proto2" => return Some("proto2".to_string()),
            "proto3" => return Some("proto3".to_string()),
            _ => {}
        }
    }

    Some(infer_syntax_from_fields(_class))
}

fn find_runtime_identifier(body: &[Statement], class_name: &str) -> Option<String> {
    let class = extract::find_class(body, class_name)?;

    for member in &class.body.body {
        let sb = match member {
            ClassElement::StaticBlock(sb) => sb,
            _ => continue,
        };
        for stmt in &sb.body {
            let expr_stmt = match stmt {
                Statement::ExpressionStatement(e) => e,
                _ => continue,
            };
            let assign = match &expr_stmt.expression {
                Expression::AssignmentExpression(a) => a,
                _ => continue,
            };

            let member_expr = match &assign.left {
                AssignmentTarget::StaticMemberExpression(m) => m,
                _ => continue,
            };
            if member_expr.property.name.as_str() != "runtime" {
                continue;
            }
            if let Expression::Identifier(id) = &assign.right {
                return Some(id.name.to_string());
            }
        }
    }
    None
}

fn resolve_runtime_value(body: &[Statement], var_name: &str, depth: u32) -> Option<String> {
    if depth > 5 {
        return None;
    }

    for stmt in body {
        let var_decl = match stmt {
            Statement::VariableDeclaration(v) => v,
            _ => continue,
        };
        for decl in &var_decl.declarations {
            let id = match &decl.id {
                BindingPattern::BindingIdentifier(id) => id,
                _ => continue,
            };
            if id.name.as_str() != var_name {
                continue;
            }
            let init = match &decl.init {
                Some(i) => i,
                None => continue,
            };

            if let Expression::Identifier(ident) = init {
                let name = ident.name.as_str();
                if name == "proto2" || name == "proto3" {
                    return Some(name.to_string());
                }
                return resolve_runtime_value(body, name, depth + 1);
            }

            if let Expression::CallExpression(call) = init
                && let Some(arg) = call.arguments.first()
                && let Some(val) = arg.as_expression().and_then(|e| util::get_string_value(e))
                && (val == "proto2" || val == "proto3")
            {
                return Some(val.to_string());
            }
        }
    }
    None
}

fn infer_syntax_from_fields(class: &Class) -> String {
    for member in &class.body.body {
        let sb = match member {
            ClassElement::StaticBlock(sb) => sb,
            _ => continue,
        };
        for stmt in &sb.body {
            let expr_stmt = match stmt {
                Statement::ExpressionStatement(e) => e,
                _ => continue,
            };
            let assign = match &expr_stmt.expression {
                Expression::AssignmentExpression(a) => a,
                _ => continue,
            };

            let member_expr = match &assign.left {
                AssignmentTarget::StaticMemberExpression(m) => m,
                _ => continue,
            };
            if member_expr.property.name.as_str() != "fields" {
                continue;
            }

            let call = match &assign.right {
                Expression::CallExpression(c) => c,
                _ => continue,
            };
            let arrow = match call.arguments.first() {
                Some(Argument::ArrowFunctionExpression(a)) => a,
                _ => continue,
            };

            let array_expr = match arrow.body.statements.first() {
                Some(Statement::ExpressionStatement(e)) => match &e.expression {
                    Expression::ArrayExpression(a) => a,
                    _ => continue,
                },
                _ => continue,
            };

            let mut has_opt = false;
            let mut non_opt_non_repeated = false;

            for el in &array_expr.elements {
                let obj = match el {
                    ArrayExpressionElement::ObjectExpression(o) => o,
                    _ => continue,
                };

                let opt_prop = obj.properties.iter().any(|p| {
                    if let ObjectPropertyKind::ObjectProperty(op) = p {
                        util::get_prop_key_name(&op.key) == Some("opt")
                    } else {
                        false
                    }
                });

                let repeated_prop = obj.properties.iter().any(|p| {
                    if let ObjectPropertyKind::ObjectProperty(op) = p {
                        util::get_prop_key_name(&op.key) == Some("repeated")
                    } else {
                        false
                    }
                });

                if opt_prop {
                    has_opt = true;
                }
                if !opt_prop && !repeated_prop {
                    non_opt_non_repeated = true;
                }
            }

            if has_opt && non_opt_non_repeated {
                return "proto2".to_string();
            }
            return "proto3".to_string();
        }
    }
    "proto3".to_string()
}
