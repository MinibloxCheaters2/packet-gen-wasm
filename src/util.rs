use oxc_ast::ast::*;

pub fn get_prop_key_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
	match key {
		PropertyKey::StaticIdentifier(id) => Some(id.name.as_str()),
		PropertyKey::Identifier(id) => Some(id.name.as_str()),
		PropertyKey::StringLiteral(s) => Some(s.value.as_str()),
		_ => None,
	}
}

pub fn get_string_value<'a>(expr: &'a Expression<'a>) -> Option<&'a str> {
	match expr {
		Expression::StringLiteral(s) => Some(s.value.as_str()),
		Expression::TemplateLiteral(t) if t.expressions.is_empty() && t.quasis.len() == 1 => Some(
			t.quasis[0]
				.value
				.cooked
				.as_deref()
				.unwrap_or(t.quasis[0].value.raw.as_str()),
		),
		_ => None,
	}
}

pub fn find_class_decl<'a>(
	body: &'a [Statement<'a>],
	name: &str,
) -> Option<&'a VariableDeclarator<'a>> {
	for stmt in body {
		if let Statement::VariableDeclaration(var_decl) = stmt {
			for decl in &var_decl.declarations {
				if let BindingPattern::BindingIdentifier(id) = &decl.id
					&& id.name.as_str() == name
				{
					return Some(decl);
				}
			}
		}
	}
	None
}

pub fn extract_class_from_init<'a>(init: &'a Expression<'a>) -> Option<&'a Class<'a>> {
	match init {
		Expression::ClassExpression(ce) => Some(ce),
		Expression::SequenceExpression(seq) => {
			if let Some(Expression::AssignmentExpression(assign)) = seq.expressions.first()
				&& let Expression::ClassExpression(ce) = &assign.right
			{
				return Some(ce);
			}
			None
		}
		_ => None,
	}
}

pub fn dereference_value<'a>(
	init: &'a Expression<'a>,
	body: &'a [Statement<'a>],
) -> Option<(&'a Expression<'a>, Option<String>)> {
	match init {
		Expression::ClassExpression(_) => Some((init, None)),
		Expression::SequenceExpression(seq) => {
			if let Some(Expression::AssignmentExpression(assign)) = seq.expressions.first() {
				let tn = if let AssignmentTarget::AssignmentTargetIdentifier(id) = &assign.left {
					Some(id.name.to_string())
				} else {
					None
				};
				Some((&assign.right, tn))
			} else {
				None
			}
		}
		Expression::Identifier(ident) => {
			let decl = find_class_decl(body, ident.name.as_str())?;
			let init2 = decl.init.as_ref()?;
			dereference_value(init2, body)
		}
		_ => None,
	}
}

pub fn dereference_decl<'a>(
	decl: &'a VariableDeclarator<'a>,
	body: &'a [Statement<'a>],
) -> Option<(&'a Expression<'a>, Option<String>)> {
	let init = decl.init.as_ref()?;
	dereference_value(init, body)
}

pub fn obj_to_pairs(obj: &ObjectExpression) -> Vec<(String, String)> {
	let mut pairs = Vec::new();
	for prop in &obj.properties {
		if let ObjectPropertyKind::ObjectProperty(p) = prop
			&& let Some(kn) = get_prop_key_name(&p.key)
			&& let Expression::Identifier(val) = &p.value
		{
			pairs.push((kn.to_string(), val.name.to_string()));
		}
	}
	pairs
}


