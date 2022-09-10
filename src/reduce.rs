use std::collections::HashMap;

use crate::{
	parse::Ident,
	resolve::{BindingIndex, Expr, ExprId, File},
};

pub fn evaluate(file: &File) -> Option<ExprId> {
	let id = file.defs.get(&Ident {
		name: "main".to_string(),
		span: 0..0,
	});

	if let Some(id) = id {
		let mut substs = file.global_bindings.clone();
		Some(reduce_expr(file, *id, &mut substs))
	} else {
		None
	}
}

fn reduce_expr(file: &File, expr: ExprId, substs: &mut HashMap<BindingIndex, ExprId>) -> ExprId {
	let ex = file.expr(expr);

	match ex {
		Expr::Fn(..) => expr,
		Expr::Apply(f, arg) => {
			let f = reduce_expr(file, *f, substs);
			let arg = reduce_expr(file, *arg, substs);
			match file.expr(f) {
				Expr::Fn(b, body) => {
					substs.insert(*b, arg);
					reduce_expr(file, *body, substs)
				},
				_ => unreachable!(),
			}
		},
		Expr::Ref(bind) => {
			let x = substs[bind];
			reduce_expr(file, x, substs)
		},
	}
}
