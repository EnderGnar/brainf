use std::{assert_matches, fs::read_to_string};

#[derive(Debug)]
enum LexOp {
    Assign,
    Eq,
    Neq,
    Add,
    Sub,
}

#[derive(Debug)]
enum BraceTy {
    Brace,
    Bracket,
    Curly,
}

fn brace_ty(c: &char) -> BraceTy {
    use BraceTy::*;
    match c {
        '(' => Brace,
        '[' => Bracket,
        '{' => Curly,
        _ => unreachable!()
    }
}

#[derive(Debug)]
enum LexNode {
    Type(String),
    Var(String),
    Value(i32),
    While,
    If,
    Return,
    Op(LexOp),
    Brace(BraceTy, LexTree),
    Semicolon,
}

#[derive(Debug, Default)]
struct LexTree {
    nodes: Vec<LexNode>
}


#[derive(Debug)]
struct Array {
    name: String,
    len: u32,
}

#[derive(Debug)]
enum Expression {
    Val(i32),
    Var(String),
    Access(String, Box<Expression>),
    Op {a: Box<Expression>, b: Box<Expression>, op: LexOp},
    Call {function: String, args: Vec<Expression>},
}

#[derive(Debug)]
enum Stmt {
    Assign {l: Expression, r: Expression},
    If {condition: Expression, body: Body},
    While {condition: Expression, body: Body},
    Return(Option<Expression>),
    Call {call: Expression},
}


#[derive(Debug)]
struct Body {
    stmts: Vec<Stmt>,
}
#[derive(Debug)]
struct Function {
    name: String,
    body: Body
}

#[derive(Debug)]
pub struct AST {
    arrays: Vec<Array>,
    funcs: Vec<Function>,
}

// TODO Handle semicolon.
fn consume(chars: &[char]) -> Vec<LexNode> {
    let semi = chars[chars.len()-1] == ';';

    if semi && chars.len() == 1 {
        return vec![LexNode::Semicolon];
    }

    let strlen = if semi {chars.len()-1} else {chars.len()};
    let str: String = chars[..strlen].iter().collect();
    use LexNode::*;
    let mut out = vec![
        match str.as_str() {
            "=" => Op(LexOp::Assign),
            "==" => Op(LexOp::Eq),
            "!=" => Op(LexOp::Neq),
            "+" => Op(LexOp::Add),
            "-" => Op(LexOp::Sub),
            "char" => Type(str),
            "void" => Type(str),
            "while" => While,
            "if" => If,
            "return" => Return,
            "'\\n'" => Value('\n' as i32),
            "'\\t'" => Value('\t' as i32),
            "'\\\''" => Value('\'' as i32),
            _ => {
                if chars[0] == '\'' {
                    if str.len() == 3 {
                        Value(chars[1] as i32)
                    }
                    else {
                        panic!("Bad value {str}")
                    }
                }
                else if let Ok(val) = str.parse::<i32>() {
                    Value(val)
                } else {
                    Var(str)
                }
            }
        }
    ];
    if semi {
        out.push(Semicolon);
    }
    out
}

const BRACES: [char; 3] = ['(', '[', '{'];
const ENDS:   [char; 3] = [')', ']', '}'];

fn lex_c(chars: &[char]) -> (LexTree, usize) {
    let mut tree = LexTree::default();
    let mut current = Vec::new();
    let mut idx = 0;

    println!("ENTER");
    
    while idx < chars.len() {
        if chars[idx] == '#' && current != vec!['\''] {
            while chars[idx] != '\n' { idx += 1 }
        }
        else if chars[idx].is_whitespace() {
            if current.len() > 0 {
                let mut nodes = consume(&current);
                tree.nodes.append(&mut nodes);
                current.clear();
            }
            idx += 1;
        }
        else if BRACES.contains(&chars[idx]) && current != vec!['\''] {
            if current.len() > 0 {
                let mut nodes = consume(&current);
                tree.nodes.append(&mut nodes);
                current.clear();
            }

            let b = chars[idx];

            let (inner, delta) = lex_c(&chars[idx+1..]);
            idx += delta + 1;
            tree.nodes.push(LexNode::Brace(brace_ty(&b), inner))
        }
        else if ENDS.contains(&chars[idx]) && current != vec!['\''] {
            idx += 1;
            break;
        } else {
            current.push(chars[idx]);
            idx += 1;
        }
    }

    if current.len() > 0 {
        let mut nodes = consume(&current);
        tree.nodes.append(&mut nodes);
        current.clear();
    }



    (tree, idx)
}

fn parse_function(fn_ty: &String, name: &String, args: &LexTree, body: &LexTree) -> Function {
    todo!();
}


fn parse(lt: LexTree) -> AST {
    let mut arrays = Vec::new();
    let mut funcs = Vec::new();

    let nodes = lt.nodes;
    let mut i = 0;
    while i+3 < nodes.len() {
        let LexNode::Brace(ty, inner) = &nodes[i+2] else { panic!("INVALID CONSUME") };
        match ty {
            BraceTy::Brace => {
                // Function
                let LexNode::Type(fn_ty) = &nodes[i] else { panic!("INVALID CONSUME") };
                let LexNode::Var(name) = &nodes[i+1] else { panic!("INVALID CONSUME") };
                let LexNode::Brace(bty, body) = &nodes[i+3] else { panic!("INVALID CONSUME") };
                funcs.push(parse_function(fn_ty, name, inner, body));
                assert_matches!(bty, BraceTy::Curly);
            },
            BraceTy::Bracket => {
                // Array
                
            },
            BraceTy::Curly => panic!("INVALID CONSUME"),
        }
    }

    if i < nodes.len() {
        panic!("NOT EVERYTHING CONSUMED!");
    }

    AST { funcs, arrays }
}


pub fn parse_c_file(filename: &str) -> AST {
    let str = read_to_string(filename).unwrap();
    let chars: Vec<_> = str.chars().collect();
    let (lextree, _) = lex_c(&chars);
    dbg!(&lextree);
    parse(lextree)
}
