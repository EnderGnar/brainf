use crate::{ast::{AST, Body, Expression, Function, LexOp}, bbir::{Array, BasicBlock, Finalizer, Label, OpTy, Operation, Program}};

struct BBBuilder {
    label: Label,
    params: Vec<String>,
    operations: Vec<Operation>,
    finalizer: Option<Finalizer>
}

impl BBBuilder {
    fn new(label: Label) -> Self {
        Self {
            label,
            params: Vec::new(),
            operations: Vec::new(),
            finalizer: None,
        }
    }
}

impl TryInto<BasicBlock> for BBBuilder {
    type Error = ();

    fn try_into(self) -> Result<BasicBlock, Self::Error> {
        let Some(finalizer) = self.finalizer else { return Err(()) };
        Ok(BasicBlock {
            label: self.label,
            params: self.params,
            operations: self.operations,
            finalizer,
        })
    }
}

// Compile an expression and tell me, where this expression can be found in the end.
fn compile_exp_rec(exp: Expression, target_var: String, mut live_var: usize) -> Vec<Operation> {
    let fresh_var = |lv: &mut usize| {
        *lv += 1;
        format!("_exp_{lv}")
    };
    match exp {
        Expression::Val(val) => {
            vec![Operation { op_type: OpTy::Set, args: vec![target_var, val.to_string()] }]
        },
        Expression::Var(var) => {
            vec![Operation { op_type: OpTy::Set, args: vec![target_var, var] }]
        },
        Expression::Access(arr, box Expression::Var(idx)) => {
            vec![Operation { op_type: OpTy::Load, args: vec![arr, idx, target_var] }]
        },
        Expression::Op { box a, box b, op } => {
            let mut ops = vec![];
            let a_var = if let Expression::Var(v) = a {v} else {
                let v = fresh_var(&mut live_var);
                let mut a_ops = compile_exp_rec(a, v.clone(), live_var);
                ops.append(&mut a_ops);
                v
            };

            let b_var = if let Expression::Var(v) = b {v} else {
                let v = fresh_var(&mut live_var);
                let mut b_ops = compile_exp_rec(b, v.clone(), live_var);
                ops.append(&mut b_ops);
                v
            };

            match op {
                LexOp::Eq => {
                    ops.push( Operation { op_type: OpTy::Sub, args: vec![target_var.clone(), a_var, b_var] } );
                    ops.push( Operation { op_type: OpTy::IsZero, args: vec![target_var.clone(), target_var] } );
                },
                LexOp::Neq => {
                    ops.push( Operation { op_type: OpTy::Sub, args: vec![target_var.clone(), a_var, b_var] } );
                    ops.push( Operation { op_type: OpTy::IsZero, args: vec![target_var.clone(), target_var.clone()] } );
                    ops.push( Operation { op_type: OpTy::IsZero, args: vec![target_var.clone(), target_var] } );
                },
                LexOp::Add => {
                    ops.push(
                        Operation { op_type: OpTy::Sub, args: vec![target_var, a_var, b_var] }
                    )
                },
                LexOp::Sub => {
                    ops.push(
                        Operation { op_type: OpTy::Add, args: vec![target_var, a_var, b_var] }
                    )
                },
                LexOp::Assign => panic!("ASSIGN IN OP"),
            }

            ops
        },
        x => panic!("BAD EXPRESSION {x:?}")
    }
}

fn compile_exp(exp: Expression, target_var: String) -> Vec<Operation> {
    compile_exp_rec(exp, target_var, 0)
}

fn compile_assign(l: Expression, r: Expression) -> Vec<Operation> {
    match l {
        Expression::Var(var) => {
            compile_exp(r, var)
        },
        Expression::Access(arr, box Expression::Var(idx)) => {
            let tmp = "_tmp_arr".to_string();
            let mut rops = compile_exp(r, tmp.clone());
            rops.push(Operation { op_type: OpTy::Store, args: vec![arr, idx, tmp] });
            rops
        },
        x => panic!("BAD LVALUE {x:?}")
    }
}

fn compile_body(body: Body, bbs: &mut Vec<BasicBlock>, label_num: &mut usize, name: &str, mut builder: BBBuilder) -> Option<BBBuilder> {
    let always_zero = "$root";
    for stmt in body.stmts {
        match stmt {
            crate::ast::Stmt::Assign { l, r:Expression::Call { function, args } } => {
                let next_n = *label_num;
                let next_label = Label::try_from(format!("{name}:nx{next_n}").as_str()).unwrap();
                *label_num += 1;
                let next_builder = BBBuilder::new(next_label.clone());
                
                let call_label = Label::try_from(format!("{function}:in").as_str()).unwrap();
                let args = args.into_iter().map(|arg| {
                    let Expression::Var(arg) = arg else { panic!("BAD ARG {arg:?}") };
                    arg
                }).collect();

                builder.finalizer = Some(Finalizer::Call { ret_label: next_label, call_label, args });
                bbs.push(builder.try_into().unwrap());
                builder = next_builder;

                let ret = Expression::Var("ret".to_string());
                let mut operations = compile_assign(l, ret);
                builder.operations.append(&mut operations);
            },
            crate::ast::Stmt::Assign { l, r } => {
                let mut operations = compile_assign(l, r);
                builder.operations.append(&mut operations);
            },
            crate::ast::Stmt::If { condition, body } => {
                let cond_name = "cond".to_string();
                let mut ops = compile_exp(condition, cond_name.clone());
                builder.operations.append(&mut ops);

                let if_n = *label_num;
                let if_label = Label::try_from(format!("{name}:if{if_n}").as_str()).unwrap();
                *label_num += 1;
                let if_builder = BBBuilder::new(if_label.clone());
                
                let if_builder = compile_body(body, bbs, label_num, name, if_builder);
                
                let next_n = *label_num;
                let next_label = Label::try_from(format!("{name}:nx{next_n}").as_str()).unwrap();
                *label_num += 1;
                let next_builder = BBBuilder::new(next_label.clone());
                
                if let Some(mut if_builder) = if_builder {
                    if_builder.finalizer = Some(Finalizer::Branch { condition: always_zero.to_string(), positive: next_label.clone(), negative: next_label.clone() });
                    bbs.push(if_builder.try_into().unwrap());
                }

                builder.finalizer = Some(Finalizer::Branch { condition: cond_name, positive: if_label, negative: next_label });
                bbs.push(builder.try_into().unwrap());
                builder = next_builder;
            },
            crate::ast::Stmt::While { condition, body } => {
                let cond_name = "_cond".to_string();
                let mut ops = compile_exp(condition, cond_name.clone());
                let mut ops_clone = ops.clone();
                builder.operations.append(&mut ops);

                let while_n = *label_num;
                let while_label = Label::try_from(format!("{name}:while{while_n}").as_str()).unwrap();
                *label_num += 1;
                let while_builder = BBBuilder::new(while_label.clone());
                
                let while_builder = compile_body(body, bbs, label_num, name, while_builder);
                
                let next_n = *label_num;
                let next_label = Label::try_from(format!("{name}:nx{next_n}").as_str()).unwrap();
                *label_num += 1;
                let next_builder = BBBuilder::new(next_label.clone());
                
                if let Some(mut while_builder) = while_builder {
                    while_builder.operations.append(&mut ops_clone);
                    while_builder.finalizer = Some(Finalizer::Branch { condition: cond_name.clone(), positive: while_label.clone(), negative: next_label.clone() });
                    bbs.push(while_builder.try_into().unwrap());
                }

                builder.finalizer = Some(Finalizer::Branch { condition: cond_name, positive: while_label, negative: next_label });
                bbs.push(builder.try_into().unwrap());
                builder = next_builder;
            },
            crate::ast::Stmt::Return(expression) => {
                let ret_value = if let Some(expression) = expression {
                    let ret_name = "_ret".to_string();
                    let mut ops = compile_exp(expression, ret_name.clone());
                    builder.operations.append(&mut ops);
                    Some(ret_name)
                }
                else {
                    None
                };
                builder.finalizer = Some(Finalizer::Return { ret_value });
                bbs.push(builder.try_into().unwrap());
                return None;
            },
            crate::ast::Stmt::Call { call:Expression::Call { function, args } } => {
                let next_n = *label_num;
                let next_label = Label::try_from(format!("{name}:nx{next_n}").as_str()).unwrap();
                *label_num += 1;
                let next_builder = BBBuilder::new(next_label.clone());
                
                let call_label = Label::try_from(format!("{function}:in").as_str()).unwrap();
                let args = args.into_iter().map(|arg| {
                    let Expression::Var(arg) = arg else { panic!("BAD ARG {arg:?}") };
                    arg
                }).collect();

                builder.finalizer = Some(Finalizer::Call { ret_label: next_label, call_label, args });
                bbs.push(builder.try_into().unwrap());
                builder = next_builder;
            },
            stmt => panic!("BAD STATEMENT {stmt:?}")
        }
    }

    Some(builder)
}

fn compile_func(func: Function) -> Vec<BasicBlock> {
    let name = func.name;
    let initial_label = Label::try_from(format!("{name}:in").as_str()).unwrap();
    let mut label_num = 1;
    let mut bbs = Vec::new();

    let builder = BBBuilder {
        label: initial_label,
        params: func.params,
        operations: Vec::new(),
        finalizer: None,
    };

    let open_builder = compile_body(func.body, &mut bbs, &mut label_num, &name, builder);
    assert!(open_builder.is_none());

    bbs
}

pub fn compile_ast(ast: AST) -> Program {
    let mut bbs = Vec::new();
    
    for func in ast.funcs {
        let mut f_bbs = compile_func(func);
        bbs.append(&mut f_bbs);
    }


    let arrs = ast.arrays.into_iter().map(
        |arr| Array { name: arr.name, len: arr.len.try_into().unwrap() }
    ).collect();
    
    Program {
        arrs,
        bbs
    }
}
