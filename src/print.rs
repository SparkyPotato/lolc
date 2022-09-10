use pretty::RcDoc;

use crate::parse::{Expr, ExprId, File};

pub fn print(file: &File) -> String {
	let mut v = Vec::new();

	for id in file.def_order.iter() {
		let expr = file.defs[id];
		let doc = RcDoc::as_string(&id.name)
			.append(RcDoc::space())
			.append(RcDoc::text("="))
			.append(RcDoc::space())
			.append(expr_doc(file, &expr, false))
			.append(RcDoc::hardline());

		doc.render(120, &mut v).unwrap();
	}

	String::from_utf8(v).unwrap()
}

pub fn print_expr(file: &File, expr: ExprId) -> String {
	let mut v = Vec::new();

	expr_doc(file, &expr, true).render(120, &mut v).unwrap();

	String::from_utf8(v).unwrap()
}

fn expr_doc(file: &File, expr: &ExprId, should_minify: bool) -> RcDoc<'static> {
	match file.inverse_defs.get(expr) {
		Some(ident) if should_minify => RcDoc::as_string(&ident.name),
		_ => {
			let expr = file.expr(*expr);
			match expr {
				Expr::Fn(args, val) => RcDoc::text("λ")
					.append(RcDoc::intersperse(
						args.iter().map(|arg| RcDoc::as_string(&arg.name)),
						RcDoc::text(" "),
					))
					.append(RcDoc::text(" . ").append(expr_doc(file, val, true))),
				Expr::Apply(f, arg) => {
					let doc = expr_doc(file, f, true);
					if matches!(file.expr(*f), Expr::Fn(..)) {
						RcDoc::text("(").append(doc).append(RcDoc::text(")"))
					} else {
						doc
					}
					.append(RcDoc::text(" "))
					.append(expr_doc(file, arg, true))
				},
				Expr::Ref(ident) => RcDoc::as_string(&ident.name),
			}
		},
	}
}
