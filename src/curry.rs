use std::collections::HashMap;

use crate::{parse, parse::Ident};

#[derive(Debug)]
pub struct File {
	pub defs: HashMap<Ident, ExprId>,
	pub inverse_defs: HashMap<ExprId, Ident>,
	pub exprs: Vec<Expr>,
}

impl File {
	pub fn insert_expr(&mut self, expr: Expr) -> ExprId {
		let id = self.exprs.len();
		self.exprs.push(expr);
		ExprId(id as _)
	}

	pub fn insert_def(&mut self, name: Ident, expr: ExprId) {
		self.defs.insert(name.clone(), expr);
		self.inverse_defs.insert(expr, name);
	}

	pub fn expr_mut(&mut self, id: ExprId) -> &mut Expr { &mut self.exprs[id.0 as usize] }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ExprId(u32);

#[derive(Clone, Debug)]
pub enum Expr {
	Fn(Ident, ExprId),
	Apply(ExprId, ExprId),
	Ref(Ident),
}

pub fn curry(mut file: parse::File) -> File {
	let mut f = File {
		defs: HashMap::new(),
		inverse_defs: HashMap::new(),
		exprs: Vec::new(),
	};

	let defs = file.defs.clone();
	for (id, def) in defs {
		let expr = curry_expr(&mut f, &mut file, def);
		f.insert_def(id, expr);
	}

	f
}

fn curry_expr(file: &mut File, p_file: &mut parse::File, expr: parse::ExprId) -> ExprId {
	let expr = p_file.expr_mut(expr);

	match expr {
		parse::Expr::Fn(name, body) => {
			let name = std::mem::take(name);
			let body = *body;

			let mut body = curry_expr(file, p_file, body);
			for name in name.into_iter().rev() {
				body = file.insert_expr(Expr::Fn(name, body));
			}
			body
		},
		parse::Expr::Apply(a, b) => {
			let a = *a;
			let b = *b;
			let f = curry_expr(file, p_file, a);
			let x = curry_expr(file, p_file, b);
			file.insert_expr(Expr::Apply(f, x))
		},
		parse::Expr::Ref(name) => file.insert_expr(Expr::Ref(std::mem::take(name))),
	}
}
