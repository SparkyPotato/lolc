use std::collections::HashMap;

use crate::{
	parse::Ident,
	resolve::{BindingIndex, Expr, ExprId, File},
};

pub fn evaluate(file: &mut File) -> Option<ExprId> {
	let mut substs = file.global_bindings.clone();

	let mut def = file.defs.clone();
	let mut inverse = file.inverse_defs.clone();
	for (name, expr) in def.iter_mut() {
		let id = reduce_expr(file, *expr, &mut substs);
		*expr = id;
		if name.name != "main" {
			inverse.insert(id, name.clone());
		}
	}

	file.defs = def;
	file.inverse_defs = inverse;

	file.defs
		.get(&Ident {
			name: "main".to_string(),
			span: 0..0,
		})
		.copied()
}

fn reduce_expr(file: &mut File, expr: ExprId, substs: &mut HashMap<BindingIndex, ExprId>) -> ExprId {
	stacker::maybe_grow(32 * 1024, 1024 * 1024, || {
		let ex = file.expr(expr);

		match ex {
			Expr::Fn(b, body) => {
				let b = *b;
				let body = reduce_expr(file, *body, substs);
				file.insert_expr(Expr::Fn(b, body))
			},
			Expr::Apply(f, arg) => {
				let f = *f;
				let arg = *arg;

				let f = reduce_expr(file, f, substs);
				let arg = reduce_expr(file, arg, substs);
				match file.expr(f) {
					Expr::Fn(b, body) => {
						let b = *b;
						substs.insert(b, arg);
						reduce_expr(file, *body, substs)
					},
					_ => file.insert_expr(Expr::Apply(f, arg)),
				}
			},
			Expr::Ref(bind) => {
				if let Some(x) = substs.get(bind) {
					reduce_expr(file, *x, substs)
				} else {
					expr
				}
			},
		}
	})
}
