use crate::bbir::{compile_program, parse_bbir_file};

mod bbir;

fn main() {
    let prog = parse_bbir_file("bbir_example/math.bbir");
    let bf = compile_program(prog);
    println!("{bf}");
}
