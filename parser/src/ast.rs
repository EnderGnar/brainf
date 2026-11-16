use std::{assert_matches, fs::read_to_string};

#[derive(Debug, Clone, Copy)]
pub(crate) enum LexOp {
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
    Comma,
}

#[derive(Debug, Default)]
struct LexTree {
    nodes: Vec<LexNode>
}


#[derive(Debug)]
pub(crate) struct Array {
    pub(crate) name: String,
    pub(crate) len: u32,
}

#[derive(Debug)]
pub(crate) enum Expression {
    Val(i32),
    Var(String),
    Access(String, Box<Expression>),
    Op {a: Box<Expression>, b: Box<Expression>, op: LexOp},
    Call {function: String, args: Vec<Expression>},
}

#[derive(Debug)]
pub(crate) enum Stmt {
    Assign {l: Expression, r: Expression},
    If {condition: Expression, body: Body},
    While {condition: Expression, body: Body},
    Return(Option<Expression>),
    Call {call: Expression},
}


#[derive(Debug)]
pub(crate) struct Body {
    pub(crate) stmts: Vec<Stmt>,
}
#[derive(Debug)]
pub(crate) struct Function {
   pub(crate) name: String,
   pub(crate) params: Vec<String>,
   pub(crate) body: Body
}

#[derive(Debug)]
pub(crate) struct AST {
    pub(crate) arrays: Vec<Array>,
    pub(crate) funcs: Vec<Function>,
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
            "," => Comma,
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

fn matching_chars(a: char, b: char) -> bool {
    let varish = |c: char| 'a' <= c && c <= 'z' || 'A' <= c && c <= 'Z' || '0' <= c && c <= '9' || c == '_';
    varish(a) == varish(b)
}

fn lex_c(chars: &[char]) -> (LexTree, usize) {
    let mut tree = LexTree::default();
    let mut current = Vec::new();
    let mut idx = 0;
    
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
            if current.len() > 0 && !matching_chars(current[0], chars[idx]) {
                let mut nodes = consume(&current);
                tree.nodes.append(&mut nodes);
                current.clear();
            }
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

fn parse_params(lt: &LexTree) -> Vec<String> {
    for (i, node) in lt.nodes.iter().enumerate() {
        match i%3 {
            0 => assert_matches!(node, LexNode::Type(_)),
            1 => assert_matches!(node, LexNode::Var(_)),
            2 => assert_matches!(node, LexNode::Comma),
            _ => unreachable!()
        }
    }

    let mut iterator = lt.nodes.iter().skip(1);
    let mut params = Vec::new();
    while let Some(LexNode::Var(var)) = iterator.next() {
        params.push(var.clone());
        iterator.next();
        iterator.next();
    }
    params
}

// Expressions are super simplfied from C.
// We assume at most 1 operator.
fn parse_exp(nodes: &[LexNode]) -> Expression {
    if let Some(op_pos) = nodes.iter().position(|n| matches!(n, LexNode::Op(_))) {
        let a = parse_exp(&nodes[..op_pos]);
        let b = parse_exp(&nodes[op_pos+1..]);
        let LexNode::Op(op) = nodes[op_pos] else { unreachable!() };
        Expression::Op { a: Box::new(a), b: Box::new(b), op }
    }
    else {
        match (nodes.len(), &nodes[0]) {
            (1, LexNode::Var(str)) => {
                Expression::Var(str.clone())
            },
            (1, LexNode::Value(val)) => {
                Expression::Val(*val)
            },
            (2, LexNode::Var(name)) => {
                let LexNode::Brace(br_ty, inner) = &nodes[1] else { panic!("NO Expression {nodes:?}") };
                match  br_ty {
                    BraceTy::Brace => {
                        Expression::Call { function: name.clone(), args: parse_args(inner) }
                    },
                    BraceTy::Bracket => {
                        Expression::Access(name.clone(), Box::new(parse_exp(&inner.nodes)))
                    },
                    BraceTy::Curly => panic!("NO Expression {nodes:?}")
                }
            },
            _ => panic!("NO Expression {nodes:?}")   
        }
    }
}


fn next_comma(nodes: &[LexNode]) -> Option<usize> {
    nodes.iter().position(|n| matches!(n, LexNode::Comma))
}

fn parse_args(lt: &LexTree) -> Vec<Expression> {
    let nodes = &lt.nodes;
    let mut args = Vec::new();
    let mut i = 0;

    while i < nodes.len() {
        if let Some(comma) = next_comma(&nodes[i..]) {
            args.push(parse_exp(&nodes[i..i+comma]));
            i += comma + 1;
        } else {
            args.push(parse_exp(&nodes[i..]));
            i = nodes.len()
        }
    }

    args
}

fn next_semicolon(nodes: &[LexNode]) -> usize {
    nodes.iter().position(|n| matches!(n, LexNode::Semicolon)).unwrap()
}

fn parse_stmts(lt: &LexTree) -> Body {
    let nodes = &lt.nodes;
    let mut stmts = Vec::new();
    let mut i = 0;

    while i < nodes.len()  {
        match &nodes[i] {
            LexNode::While => {
                let LexNode::Brace(BraceTy::Brace, con) = &nodes[i+1] else { panic!("No condition") };
                let LexNode::Brace(BraceTy::Curly, bod) = &nodes[i+2] else { panic!("No body") };
                stmts.push(Stmt::While { condition: parse_exp(&con.nodes), body: parse_stmts(bod) });
                i += 3;
            },
            LexNode::If => {
                let LexNode::Brace(BraceTy::Brace, con) = &nodes[i+1] else { panic!("No condition") };
                let LexNode::Brace(BraceTy::Curly, bod) = &nodes[i+2] else { panic!("No body") };
                stmts.push(Stmt::If { condition: parse_exp(&con.nodes), body: parse_stmts(bod) });
                i += 3;
            },
            LexNode::Var(str) => {
                if i+1 >= nodes.len() {panic!("Where is the rest? {str}")}
                match &nodes[i+1] {
                    LexNode::Op(LexOp::Assign) => {
                        let semi_idx = i + next_semicolon(&nodes[i..]);
                        stmts.push(Stmt::Assign { l: parse_exp(&nodes[i..i+1]), r: parse_exp(&nodes[i+2..semi_idx]) });
                        i = semi_idx + 1;
                    }
                    LexNode::Brace(BraceTy::Brace, _) => {
                        assert_matches!(&nodes[i+2], LexNode::Semicolon);
                        stmts.push(
                            Stmt::Call { call: parse_exp(&nodes[i..i+2]) }
                        );
                        i += 3;
                    }
                    LexNode::Brace(BraceTy::Bracket, _index) => {
                        assert_matches!(&nodes[i+2], LexNode::Op(LexOp::Assign));
                        let semi_idx = i + next_semicolon(&nodes[i..]);

                        stmts.push(Stmt::Assign { l: parse_exp(&nodes[i..i+2]), r: parse_exp(&nodes[i+3..semi_idx]) });
                        i = semi_idx + 1;
                    }
                    x => {
                        let slice = &nodes[i..];
                        panic!("UNKNOWN STMT {x:?} {slice:?}")
                    }
                }
            },
            LexNode::Return => {
                let semi_idx = i + next_semicolon(&nodes[i..]);
                if semi_idx == i + 1 {
                    stmts.push(Stmt::Return(None));
                } else {
                    stmts.push(Stmt::Return(Some(parse_exp(&nodes[i+1..semi_idx]))));
                }
                i = semi_idx + 1;
            }
            // HACKY: Should only allow an assign next.
            LexNode::Type(_) => {
                i += 1;
            }
            x => panic!("UNKNOW STMT START {x:?}")
        }
    }


    Body {
        stmts,
    }
}

fn parse_function(name: &String, params: &LexTree, body: &LexTree) -> Function {
    Function {
        name: name.clone(),
        params: parse_params(params),
        body: parse_stmts(body),
    }
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
                assert_matches!(&nodes[i], LexNode::Type(_));
                let LexNode::Var(name) = &nodes[i+1] else { panic!("INVALID CONSUME") };
                let LexNode::Brace(BraceTy::Curly, body) = &nodes[i+3] else { panic!("INVALID CONSUME") };
                funcs.push(parse_function(name, inner, body));
                i += 4;
            },
            BraceTy::Bracket => {
                // Array
                let LexNode::Var(name) = &nodes[i+1] else { panic!("INVALID CONSUME") };
                assert_matches!(&nodes[i+3], LexNode::Semicolon);
                assert_eq!(inner.nodes.len(), 1);
                let LexNode::Value(len) = &inner.nodes[0] else { panic!("INVALID LENGTH") };
                arrays.push( Array { name: name.to_string(), len: (*len as u32) });  
                i += 4;              
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
    parse(lextree)
}
