use std::{cell::RefCell, collections::HashMap, hash::Hash, ops::Range};

use ariadne::{Label, Report, ReportKind};
use chumsky::{error::SimpleReason, prelude::*, Parser};

#[derive(Clone, Eq, Debug, Default)]
pub struct Ident {
	pub name: String,
	pub span: Range<usize>,
}

impl Hash for Ident {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) { self.name.hash(state); }
}

impl PartialEq for Ident {
	fn eq(&self, other: &Self) -> bool { self.name == other.name }
}

#[derive(Debug)]
pub struct File {
	pub defs: HashMap<Ident, ExprId>,
	pub inverse_defs: HashMap<ExprId, Ident>,
	pub exprs: Vec<Expr>,
	pub def_order: Vec<Ident>,
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

	pub fn expr_mut(&mut self, id: ExprId) -> &mut Expr { &mut self.exprs[id.0 as usize] }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ExprId(u32);

#[derive(Clone, Debug)]
pub enum Expr {
	Fn(Vec<Ident>, ExprId),
	Apply(ExprId, ExprId),
	Ref(Ident),
}

pub fn parse(s: &str) -> (File, Vec<Report>) {
	let file = RefCell::new(File {
		defs: HashMap::new(),
		inverse_defs: HashMap::new(),
		exprs: Vec::new(),
		def_order: Vec::new(),
	});

	let space = filter(|c| {
		char::is_whitespace(*c) && !matches!(*c, '\n' | '\x0B' | '\x0C' | '\u{0085}' | '\u{2028}' | '\u{2029}')
	})
	.repeated();

	let lambda = just("\\").or(just("λ")).padded();
	let ident = filter(|c| !matches!(*c, '\\' | 'λ' | '.' | '(' | ')' | '=') && !c.is_whitespace())
		.repeated()
		.at_least(1)
		.padded_by(space)
		.map_with_span(|name, span| Ident {
			name: name.into_iter().collect(),
			span,
		});
	let tok = |s| just(s).padded_by(space);

	let expr = recursive(|expr| {
		let atom = tok('(')
			.ignore_then(expr.clone())
			.then_ignore(tok(')'))
			.or(ident.clone().map(|x| file.borrow_mut().insert_expr(Expr::Ref(x))));

		atom.repeated()
			.at_least(1)
			.map(|list| {
				let mut iter = list.into_iter();
				let mut expr = iter.next().unwrap();

				for next in iter {
					expr = file.borrow_mut().insert_expr(Expr::Apply(expr, next));
				}

				expr
			})
			.or(lambda
				.ignore_then(ident.clone().repeated())
				.then_ignore(tok('.'))
				.then(expr.clone())
				.map(|(param, expr)| file.borrow_mut().insert_expr(Expr::Fn(param, expr))))
	});

	let def = ident
		.then_ignore(tok('='))
		.then(expr)
		.then_ignore(end().or(text::newline().repeated().at_least(1).ignored()))
		.map(|(name, expr)| {
			let mut file = file.borrow_mut();
			file.def_order.push(name.clone());
			file.insert_def(name, expr);
		});

	let (_, errors): (_, Vec<Simple<_, _>>) = def.repeated().then_ignore(end()).parse_recovery(s);

	let mut diagnostics = Vec::new();
	for error in errors {
		let span = error.span();
		let mut builder = Report::build(ReportKind::Error, (), span.start);

		match error.reason() {
			SimpleReason::Custom(s) => builder.set_message(s),
			SimpleReason::Unexpected => {
				builder.set_message(match error.found() {
					Some(tok) => match error.label() {
						Some(label) => format!("unexpected `{}` while parsing {}", tok, label),
						None => format!("unexpected `{}`", tok),
					},
					None => "unexpected `<eof>`".into(),
				});

				match error.expected().len() {
					0 => builder.add_label(Label::new(span)),
					1 => builder.add_label(Label::new(span).with_message(match error.expected().next().unwrap() {
						Some(tok) => format!("expected `{}`", tok),
						None => "expected `<eof>`".into(),
					})),
					_ => builder.add_label(Label::new(span).with_message(format!(
						"expected one of {}",
						error
							.expected()
							.map(|tok| match tok {
								Some(tok) => format!("`{}`", tok),
								None => "`<eof>`".into(),
							})
							.collect::<Vec<_>>()
							.join(", ")
					))),
				}
			},
			SimpleReason::Unclosed { delimiter, span } => match error.label() {
				Some(label) => {
					builder.set_message(format!("unclosed `{}` while parsing {}", delimiter, label));
					builder.add_label(Label::new(span.clone()))
				},
				None => {
					builder.set_message(format!("unclosed `{}`", delimiter));
					builder.add_label(Label::new(span.clone()))
				},
			},
		}

		diagnostics.push(builder.finish());
	}

	(file.into_inner(), diagnostics)
}
