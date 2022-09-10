use std::{collections::HashMap, ops::Range};

use ariadne::{Label, Report, ReportKind};

use crate::{curry, parse, parse::Ident};

#[derive(Debug)]
pub struct File {
	pub defs: HashMap<Ident, ExprId>,
	pub global_bindings: HashMap<BindingIndex, ExprId>,
	pub inverse_defs: HashMap<ExprId, Ident>,
	pub exprs: Vec<Expr>,
	pub index_to_name: HashMap<BindingIndex, Ident>,
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

	pub fn expr(&self, id: ExprId) -> &Expr { &self.exprs[id.0 as usize] }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ExprId(u32);
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct BindingIndex(u32);

#[derive(Clone, Debug)]
pub enum Expr {
	Fn(BindingIndex, ExprId),
	Apply(ExprId, ExprId),
	Ref(BindingIndex),
}

pub fn resolve(mut file: curry::File) -> (File, Vec<Report>) {
	let mut f = File {
		defs: HashMap::new(),
		global_bindings: HashMap::new(),
		inverse_defs: HashMap::new(),
		exprs: Vec::new(),
		index_to_name: HashMap::new(),
	};
	let mut diagnostics = Vec::new();

	let mut incr = 0;
	let mut stack = file
		.defs
		.iter()
		.map(|x| {
			let id = BindingIndex(incr);
			incr += 1;
			f.index_to_name.insert(id, x.0.clone());
			(x.0.name.clone(), (id, x.0.span.clone()))
		})
		.collect();

	let defs = file.defs.clone();
	for (id, def) in defs {
		if let Some(expr) = resolve_expr(&mut f, &mut file, def, &mut stack, &mut diagnostics, &mut incr) {
			f.global_bindings.insert(stack[&id.name].0, expr);
			f.insert_def(id, expr);
		}
	}

	(f, diagnostics)
}

fn resolve_expr(
	file: &mut File, p_file: &mut curry::File, expr: curry::ExprId,
	stack: &mut HashMap<String, (BindingIndex, Range<usize>)>, diagnostics: &mut Vec<Report>, incr: &mut u32,
) -> Option<ExprId> {
	let expr = p_file.expr_mut(expr);

	match expr {
		curry::Expr::Fn(name, body) => {
			let id = BindingIndex(*incr);
			*incr += 1;
			let name = std::mem::take(name);
			file.index_to_name.insert(id, name.clone());
			if let Some(x) = stack.insert(name.name.clone(), (id, name.span.clone())) {
				diagnostics.push(
					Report::build(ReportKind::Error, (), name.span.start)
						.with_message("duplicate binding")
						.with_label(Label::new(name.span.clone()))
						.with_label(Label::new(x.1).with_message("previous binding"))
						.finish(),
				);
			}
			let body = *body;
			let body = resolve_expr(file, p_file, body, stack, diagnostics, incr)?;
			stack.remove(&name.name);

			Some(file.insert_expr(Expr::Fn(id, body)))
		},
		curry::Expr::Apply(a, b) => {
			let a = *a;
			let b = *b;
			let a = resolve_expr(file, p_file, a, stack, diagnostics, incr)?;
			let b = resolve_expr(file, p_file, b, stack, diagnostics, incr)?;
			Some(file.insert_expr(Expr::Apply(a, b)))
		},
		curry::Expr::Ref(name) => {
			if let Some(index) = stack.get(&name.name) {
				Some(file.insert_expr(Expr::Ref(index.0)))
			} else {
				diagnostics.push(
					Report::build(ReportKind::Error, (), name.span.start)
						.with_message(format!("unresolved identifier `{}`", name.name))
						.with_label(Label::new(name.span.clone()))
						.finish(),
				);
				None
			}
		},
	}
}

pub fn uncurry_expr(file: &File, id: ExprId) -> (parse::File, parse::ExprId) {
	let mut p_file = parse::File {
		defs: HashMap::new(),
		inverse_defs: HashMap::new(),
		exprs: Vec::new(),
		def_order: Vec::new(),
	};

	let expr = uncurry_expr_inner(file, &mut p_file, id);
	if let Some(ident) = file.inverse_defs.get(&id) {
		p_file.inverse_defs.insert(expr, ident.clone());
	}

	(p_file, expr)
}

fn uncurry_expr_inner(file: &File, p_file: &mut parse::File, id: ExprId) -> parse::ExprId {
	let expr = file.expr(id);

	match expr {
		Expr::Fn(name, body) => {
			let mut args = vec![file.index_to_name[name].clone()];
			let mut b = *body;
			while let Expr::Fn(name, body) = file.expr(b) {
				args.push(file.index_to_name[name].clone());
				b = *body;
			}

			let id = uncurry_expr_inner(file, p_file, b);
			p_file.insert_expr(parse::Expr::Fn(args, id))
		},
		Expr::Apply(a, b) => {
			let a = uncurry_expr_inner(file, p_file, *a);
			let b = uncurry_expr_inner(file, p_file, *b);
			p_file.insert_expr(parse::Expr::Apply(a, b))
		},
		Expr::Ref(name) => p_file.insert_expr(parse::Expr::Ref(file.index_to_name[name].clone())),
	}
}
