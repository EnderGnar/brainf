use crate::{ast::parse_c_file, bbir::{compile_program, parse_bbir_file}};

mod bbir;
mod ast;
mod ast_to_bbir;

fn main() {
    if true {
        let prog = parse_bbir_file("bbir_example/arr.bbir");
        let bf = compile_program(prog);
        println!("{bf}");
    }
    else {
        let ast = parse_c_file("../simple.c");
        dbg!(ast);
    }
}
