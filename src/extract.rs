use std::collections::{HashMap, HashSet};

use oxc_ast::ast::*;

use crate::runtime_syntax;
use crate::types::*;
use crate::util;

const SCALAR_TYPES: &[(u32, &str)] = &[
    (1, "double"),
    (2, "float"),
    (3, "int64"),
    (4, "uint64"),
    (5, "int32"),
    (6, "fixed64"),
    (7, "fixed32"),
    (8, "bool"),
    (9, "string"),
    (10, "group"),
    (11, "message"),
    (12, "bytes"),
    (13, "uint32"),
    (14, "enum"),
    (15, "sfixed32"),
    (16, "sfixed64"),
    (17, "sint32"),
    (18, "sint64"),
];

fn resolve_scalar(value: f64) -> String {
    SCALAR_TYPES
        .iter()
        .find(|(id, _)| *id == value as u32)
        .map(|(_, n)| n.to_string())
        .unwrap_or_else(|| format!("unknown_scalar_{}", value as u32))
}

fn resolve_map_value_type(obj: &ObjectExpression) -> String {
    let kind = obj.properties.iter().find_map(|p| {
        if let ObjectPropertyKind::ObjectProperty(op) = p
            && util::get_prop_key_name(&op.key) == Some("kind")
        {
            util::get_string_value(&op.value)
        } else {
            None
        }
    });
    let tp = obj.properties.iter().find_map(|p| {
        if let ObjectPropertyKind::ObjectProperty(op) = p
            && util::get_prop_key_name(&op.key) == Some("T")
        {
            Some(&op.value)
        } else {
            None
        }
    });
    if let (Some(kind), Some(tp)) = (kind, tp) {
        return resolve_field_type_str(kind, tp);
    }
    "unknown".to_string()
}

fn resolve_field_type_str(kind: &str, value: &Expression) -> String {
    match kind {
        "scalar" => match value {
            Expression::NumericLiteral(n) => resolve_scalar(n.value),
            _ => "unknown_scalar".to_string(),
        },
        "enum" => {
            if let Expression::Identifier(id) = value {
                id.name.to_string()
            } else if let Expression::CallExpression(call) = value {
                call.arguments
                    .first()
                    .and_then(|a| {
                        if let Argument::Identifier(id) = a {
                            Some(id.name.to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "unknown_enum".to_string())
            } else {
                "unknown_enum".to_string()
            }
        }
        "message" => {
            if let Expression::Identifier(id) = value {
                return id.name.to_string();
            }
            if let Expression::ArrowFunctionExpression(arrow) = value {
                for stmt in &arrow.body.statements {
                    if let Statement::ExpressionStatement(es) = stmt {
                        if let Expression::Identifier(bid) = &es.expression {
                            return bid.name.to_string();
                        }
                        if let Expression::CallExpression(call) = &es.expression
                            && let Some(Argument::Identifier(id)) = call.arguments.first()
                        {
                            return id.name.to_string();
                        }
                    }
                }
            }
            if let Expression::CallExpression(call) = value
                && let Some(Argument::Identifier(id)) = call.arguments.first()
            {
                return id.name.to_string();
            }
            "unknown_message".to_string()
        }
        "map" => "map".to_string(),
        _ => "unknown".to_string(),
    }
}

fn get_obj_prop<'a>(obj: &'a ObjectExpression<'a>, name: &str) -> Option<&'a ObjectProperty<'a>> {
    obj.properties.iter().find_map(|p| {
        if let ObjectPropertyKind::ObjectProperty(op) = p
            && util::get_prop_key_name(&op.key) == Some(name)
        {
            Some(&**op)
        } else {
            None
        }
    })
}

pub(crate) fn find_class<'a>(body: &'a [Statement<'a>], name: &str) -> Option<&'a Class<'a>> {
    for stmt in body {
        if let Statement::VariableDeclaration(vd) = stmt {
            for decl in &vd.declarations {
                if let BindingPattern::BindingIdentifier(id) = &decl.id
                    && id.name.as_str() == name
                    && let Some(init) = &decl.init
                {
                    return util::extract_class_from_init(init);
                }
            }
        }
        if let Statement::ExpressionStatement(es) = stmt
            && let Expression::AssignmentExpression(assign) = &es.expression
            && let AssignmentTarget::AssignmentTargetIdentifier(lid) = &assign.left
            && lid.name.as_str() == name
        {
            return util::extract_class_from_init(&assign.right);
        }
    }
    None
}

fn find_fields_call<'a>(body: &'a [Statement<'a>], name: &str) -> Option<&'a CallExpression<'a>> {
    let class = find_class(body, name)?;
    for member in &class.body.body {
        let ClassElement::StaticBlock(sb) = member else {
            continue;
        };
        for stmt in &sb.body {
            let Statement::ExpressionStatement(es) = stmt else {
                continue;
            };
            let Expression::AssignmentExpression(assign) = &es.expression else {
                continue;
            };
            let AssignmentTarget::StaticMemberExpression(m) = &assign.left else {
                continue;
            };
            if m.property.name.as_str() == "fields"
                && let Expression::CallExpression(call) = &assign.right
            {
                return Some(call);
            }
        }
    }
    None
}

fn extract_type_name(class: &Class) -> Option<String> {
    for member in &class.body.body {
        let ClassElement::StaticBlock(sb) = member else {
            continue;
        };
        for stmt in &sb.body {
            let Statement::ExpressionStatement(es) = stmt else {
                continue;
            };
            let Expression::AssignmentExpression(assign) = &es.expression else {
                continue;
            };
            let AssignmentTarget::StaticMemberExpression(m) = &assign.left else {
                continue;
            };
            if m.property.name.as_str() == "typeName"
                && matches!(&m.object, Expression::ThisExpression(_))
            {
                return util::get_string_value(&assign.right).map(|s| s.replace('.', "_"));
            }
        }
    }
    None
}

fn fields_to_vec(call: &CallExpression, mapping: &HashMap<String, String>) -> Vec<MappedField> {
    let Some(Argument::ArrowFunctionExpression(arrow)) = call.arguments.first() else {
        return vec![];
    };
    let Some(Statement::ExpressionStatement(es)) = arrow.body.statements.first() else {
        return vec![];
    };
    let Expression::ArrayExpression(arr) = &es.expression else {
        return vec![];
    };

    let mut out = Vec::new();
    for el in &arr.elements {
        let ArrayExpressionElement::ObjectExpression(obj) = el else {
            continue;
        };
        let no = match get_obj_prop(obj, "no") {
            Some(p) => match &p.value {
                Expression::NumericLiteral(n) => n.value,
                _ => continue,
            },
            None => continue,
        };
        let name = match get_obj_prop(obj, "name") {
            Some(p) => util::get_string_value(&p.value).unwrap_or("").to_string(),
            None => continue,
        };
        let kind = match get_obj_prop(obj, "kind") {
            Some(p) => util::get_string_value(&p.value).unwrap_or("").to_string(),
            None => continue,
        };
        let t = get_obj_prop(obj, "T").map(|p| {
            let raw = resolve_field_type_str(&kind, &p.value);
            mapping.get(&raw).cloned().unwrap_or(raw)
        });

        let repeated = get_obj_prop(obj, "repeated").is_some();
        let opt = get_obj_prop(obj, "opt").is_some();
        let oneof = get_obj_prop(obj, "oneof")
            .and_then(|p| util::get_string_value(&p.value))
            .map(|s| s.to_string());

        let map = if kind == "map" {
            let k = get_obj_prop(obj, "K").and_then(|p| match &p.value {
                Expression::NumericLiteral(n) => Some(resolve_scalar(n.value)),
                _ => None,
            });
            let v = get_obj_prop(obj, "V").map(|p| {
                if let Expression::ObjectExpression(o) = &p.value {
                    let raw = resolve_map_value_type(o);
                    mapping.get(&raw).cloned().unwrap_or(raw)
                } else {
                    "unknown".to_string()
                }
            });
            Some(MappedMap { k, v })
        } else {
            None
        };

        out.push(MappedField {
            no,
            name,
            kind,
            t,
            repeated,
            opt,
            oneof,
            map,
        });
    }
    out
}

fn extract_enums(body: &[Statement]) -> HashMap<String, Vec<MappedEnumEntry>> {
    let mut map = HashMap::new();

    for stmt in body {
        let Statement::ExpressionStatement(es) = stmt else {
            continue;
        };
        let Expression::CallExpression(call) = &es.expression else {
            continue;
        };
        let Expression::StaticMemberExpression(m) = &call.callee else {
            continue;
        };
        if m.property.name.as_str() != "setEnumType" {
            continue;
        }

        let mut args = call.arguments.iter();
        let enum_ref = match args.next() {
            Some(Argument::Identifier(id)) => id,
            _ => continue,
        };
        let name_arg = match args.next() {
            Some(a) => a,
            None => continue,
        };
        let values_arr = match args.next() {
            Some(Argument::ArrayExpression(a)) => a,
            _ => continue,
        };

        let enum_name = match name_arg {
            Argument::StringLiteral(s) => s.value.to_string(),
            Argument::TemplateLiteral(t) if t.expressions.is_empty() && t.quasis.len() == 1 => t
                .quasis[0]
                .value
                .cooked
                .as_deref()
                .unwrap_or(t.quasis[0].value.raw.as_str())
                .to_string(),
            _ => enum_ref.name.to_string(),
        };

        let mut values = Vec::new();
        for el in &values_arr.elements {
            let ArrayExpressionElement::ObjectExpression(obj) = el else {
                continue;
            };
            let no = obj.properties.iter().find_map(|p| {
                if let ObjectPropertyKind::ObjectProperty(op) = p
                    && util::get_prop_key_name(&op.key) == Some("no")
                    && let Expression::NumericLiteral(n) = &op.value
                {
                    Some(n.value)
                } else {
                    None
                }
            });
            let name = obj.properties.iter().find_map(|p| {
                if let ObjectPropertyKind::ObjectProperty(op) = p
                    && util::get_prop_key_name(&op.key) == Some("name")
                {
                    util::get_string_value(&op.value).map(|s| s.to_string())
                } else {
                    None
                }
            });
            if let (Some(no), Some(name)) = (no, name) {
                values.push(MappedEnumEntry { no, name });
            }
        }

        map.insert(enum_name, values);
    }

    map
}

struct PacketMaps {
    c_pairs: Vec<(String, String)>,
    s_pairs: Vec<(String, String)>,
    appended_pairs: Vec<(String, String)>,
}

fn find_objs_in_expr<'a>(expr: &'a Expression<'a>, out: &mut Vec<&'a ObjectExpression<'a>>) {
    match expr {
        Expression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                if let ObjectPropertyKind::ObjectProperty(p) = prop
                    && let Some(kn) = util::get_prop_key_name(&p.key)
                    && (kn.starts_with("CPacket") || kn.starts_with("SPacket"))
                {
                    out.push(obj);
                    return;
                }
            }
        }
        Expression::SequenceExpression(seq) => {
            for e in &seq.expressions {
                find_objs_in_expr(e, out);
            }
        }
        Expression::AssignmentExpression(a) => find_objs_in_expr(&a.right, out),
        _ => {}
    }
}

fn find_packet_maps(body: &[Statement]) -> PacketMaps {
    let mut c_pairs = Vec::new();
    let mut s_pairs = Vec::new();
    let mut appended_pairs = Vec::new();

    let mut objs: Vec<&ObjectExpression> = Vec::new();
    for stmt in body {
        match stmt {
            Statement::ExpressionStatement(es) => find_objs_in_expr(&es.expression, &mut objs),
            Statement::VariableDeclaration(vd) => {
                for decl in &vd.declarations {
                    if let Some(init) = &decl.init {
                        find_objs_in_expr(init, &mut objs);
                    }
                }
            }
            _ => {}
        }
    }

    for obj in &objs {
        let pairs = util::obj_to_pairs(obj);
        let has_c = pairs.iter().any(|(k, _)| k.starts_with("CPacket"));
        let has_s = pairs.iter().any(|(k, _)| k.starts_with("SPacket"));
        if has_c && !has_s {
            c_pairs = pairs;
        } else if has_s && !has_c {
            s_pairs = pairs;
        } else if has_c && has_s {
            appended_pairs = pairs;
        }
    }

    PacketMaps {
        c_pairs,
        s_pairs,
        appended_pairs,
    }
}

fn collect_deps(
    call: &CallExpression,
    scalar_names: &HashSet<String>,
    mapping: &HashMap<String, String>,
    visited: &HashSet<String>,
) -> Vec<String> {
    let Some(Argument::ArrowFunctionExpression(arrow)) = call.arguments.first() else {
        return vec![];
    };
    let Some(Statement::ExpressionStatement(es)) = arrow.body.statements.first() else {
        return vec![];
    };
    let Expression::ArrayExpression(arr) = &es.expression else {
        return vec![];
    };

    let mut deps = Vec::new();
    for el in &arr.elements {
        let ArrayExpressionElement::ObjectExpression(obj) = el else {
            continue;
        };
        let kind = match get_obj_prop(obj, "kind") {
            Some(p) => util::get_string_value(&p.value).unwrap_or(""),
            None => continue,
        };
        let tp = match get_obj_prop(obj, "T") {
            Some(p) => &p.value,
            None => continue,
        };

        let dep = match kind {
            "message" => match tp {
                Expression::Identifier(id) => id.name.to_string(),
                _ => continue,
            },
            "enum" => match tp {
                Expression::CallExpression(call) => match call.arguments.first() {
                    Some(Argument::Identifier(id)) => id.name.to_string(),
                    _ => continue,
                },
                _ => continue,
            },
            "map" => match tp {
                Expression::ObjectExpression(o) => {
                    let vt = resolve_map_value_type(o);
                    if scalar_names.contains(&vt)
                        || mapping.contains_key(&vt)
                        || visited.contains(&vt)
                    {
                        continue;
                    }
                    vt
                }
                _ => continue,
            },
            _ => continue,
        };

        if !scalar_names.contains(&dep)
            && !mapping.contains_key(&dep)
            && !visited.contains(&dep)
            && !deps.contains(&dep)
        {
            deps.push(dep);
        }
    }
    deps
}

pub fn extract_bundle(source: &str) -> Result<ParseBundleResult, String> {
    let allocator = oxc_allocator::Allocator::default();
    let source_type = oxc_span::SourceType::mjs();
    let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();

    for diag in &parsed.diagnostics {
        eprintln!("parse warning: {:?}", diag);
    }

    let body = &parsed.program.body;
    let maps = find_packet_maps(body);

    let scalar_names: HashSet<String> = SCALAR_TYPES.iter().map(|(_, n)| n.to_string()).collect();

    let mut name_mapping: HashMap<String, String> = HashMap::new();
    let mut messages: HashMap<String, MappedMessage> = HashMap::new();

    let all_pairs: Vec<&[(String, String)]> =
        vec![&maps.c_pairs, &maps.s_pairs, &maps.appended_pairs];

    for pairs in &all_pairs {
        for (_key, prop) in *pairs {
            let decl = match util::find_class_decl(body, prop) {
                Some(d) => d,
                None => continue,
            };
            let (target_expr, target_name) = match util::dereference_decl(decl, body) {
                Some(t) => t,
                None => continue,
            };
            let class = match util::extract_class_from_init(target_expr) {
                Some(c) => c,
                None => continue,
            };
            let runtime = target_name.unwrap_or_else(|| prop.clone());
            let tn = extract_type_name(class).unwrap_or_else(|| _key.clone());
            name_mapping.insert(runtime.clone(), tn.clone());

            if let Some(fc) = find_fields_call(body, &runtime) {
                let fields = fields_to_vec(fc, &name_mapping);
                let syntax = runtime_syntax::find_runtime_syntax(body, &runtime)
                    .unwrap_or_else(|| Syntax::Proto3);
                messages.insert(
                    tn.clone(),
                    MappedMessage {
                        type_name: tn,
                        syntax,
                        fields,
                    },
                );
            }
        }
    }

    let mut queue: Vec<String> = messages.keys().cloned().collect();
    let mut visited: HashSet<String> = messages.keys().cloned().collect();

    while let Some(name) = queue.pop() {
        let fc = match find_fields_call(body, &name) {
            Some(f) => f,
            None => continue,
        };
        let deps = collect_deps(fc, &scalar_names, &name_mapping, &visited);

        for dep in &deps {
            if visited.contains(dep) {
                continue;
            }

            let decl = match util::find_class_decl(body, dep) {
                Some(d) => d,
                None => continue,
            };
            let (target_expr, target_name) = match util::dereference_decl(decl, body) {
                Some(t) => t,
                None => continue,
            };
            let class = match util::extract_class_from_init(target_expr) {
                Some(c) => c,
                None => continue,
            };

            let runtime = target_name.as_deref().unwrap_or(dep);
            let tn = extract_type_name(class).unwrap_or_else(|| dep.clone());
            name_mapping.insert(runtime.to_string(), tn.clone());

            if !visited.contains(&tn)
                && let Some(fc) = find_fields_call(body, runtime)
            {
                let fields = fields_to_vec(fc, &name_mapping);
                let syntax = runtime_syntax::find_runtime_syntax(body, runtime)
                    .unwrap_or_else(|| Syntax::Proto3);
                messages.insert(
                    tn.clone(),
                    MappedMessage {
                        type_name: tn.clone(),
                        syntax,
                        fields,
                    },
                );
                visited.insert(tn.clone());
                queue.push(tn);
            }
        }
    }

    let enum_map = extract_enums(body);
    let enums: Vec<MappedEnumGroup> = enum_map
        .into_iter()
        .map(|(name, values)| MappedEnumGroup { name, values })
        .collect();
    let messages: Vec<MappedMessage> = messages.into_values().collect();

    Ok(ParseBundleResult { messages, enums })
}
