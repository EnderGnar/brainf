use std::{cell::RefCell, collections::{HashMap, HashSet}, fs::read_to_string};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Label {
    family: String,
    name: String,
}

impl TryFrom<&str> for Label {
    type Error=();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let vec: Vec<&str> = value.split(':').collect();
        assert!(vec.len() == 2);
        return Ok(Self {
            family: vec[0].to_string(),
            name: vec[1].to_string(),
        });
    }
}

#[derive(Debug)]
enum OpTy {
    Getc, Putc, Set, Add, Sub, IsZero, Or
}

impl TryFrom<&str> for OpTy {
    type Error=();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "getc" =>   Self::Getc,
            "putc" =>   Self::Putc,
            "set" =>    Self::Set,
            "add" =>    Self::Add,
            "sub" =>    Self::Sub,
            "iszero" => Self::IsZero,
            "or" =>     Self::Or,
            _ => return Err(())
        })
    }
}

#[derive(Debug)]
struct Operation {
    op_type: OpTy,
    args: Vec<String>
}

#[derive(Debug)]
enum Finalizer {
    Branch {
        condition: String,
        positive: Label,
        negative: Label,
    },
    Call {
        ret_label: Label,
        call_label: Label,
        args: Vec<String>
    },
    Return {
        ret_value: Option<String>
    }
}

#[derive(Debug)]
struct BasicBlock {
    label: Label,
    params: Vec<String>,
    operations: Vec<Operation>,
    finalizer: Finalizer
}

#[derive(Debug)]
pub struct Program {
    bbs: Vec<BasicBlock>
}

fn read_lines(filename: &str) -> Vec<String> {
    read_to_string(filename).unwrap().lines().map(|l| l.to_string()).collect()
}

fn lex_label(line: String) -> (Option<Label>, Vec<String>) {
    let parts: Vec<&str> = line.split_whitespace().filter(|s| s.len() > 0).collect();
    assert!(parts.len() >= 2);
    let label = Label::try_from(parts[1]).unwrap();

    let params: Vec<String> = parts[2..].iter().map(|s| s.to_string()).collect();
    return (Some(label), params);
}


fn lex_operation(line: String) -> Operation {
    let parts: Vec<&str> = line.split_whitespace().filter(|s| s.len() > 0).collect();
    assert!(parts.len() >= 2);
    let op_type = OpTy::try_from(parts[1]).unwrap();

    let args: Vec<String> = parts[2..].iter().map(|s| s.to_string()).collect();
    return Operation { op_type, args };
}

fn lex_finalizer(line: String) -> Finalizer {
    let parts: Vec<&str> = line.split_whitespace().filter(|s| s.len() > 0).collect();
    match parts[1] {
        "return" => {
            if parts.len() > 3 { panic!("Return at most 1 value.") }
            let ret_value = if parts.len() == 3 { Some(parts[2].to_string()) }  else { None };
            Finalizer::Return {
                ret_value
            }
        },
        "br" => {
            Finalizer::Branch {
                condition: parts[2].to_string(),
                positive: Label::try_from(parts[3]).unwrap(),
                negative: Label::try_from(parts[4]).unwrap()
            }
        },
        "call" => {
            Finalizer::Call {
                ret_label: Label::try_from(parts[2]).unwrap(),
                call_label: Label::try_from(parts[3]).unwrap(),
                args: parts[4..].into_iter().map(ToString::to_string).collect()
            }
        }
        _ => { panic!("Unknown Finalizer"); }
    }
}

fn lex_bbir(lines: Vec<String>) -> Vec<BasicBlock> {
    let mut bbs = Vec::<BasicBlock>::new();
    let mut label: Option<Label> = None;
    let mut params = Vec::<String>::new();
    let mut operations = Vec::<Operation>::new();

    for line in lines {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        match line.chars().nth(0).unwrap() {
            'l' => (label, params) = lex_label(line),
            'o' => operations.push(lex_operation(line)),
            'f' => {
                let finalizer = lex_finalizer(line);

                bbs.push(BasicBlock {
                    label: label.take().unwrap(),
                    params,
                    operations,
                    finalizer
                });
                params = Vec::new();
                operations = Vec::new();
            },
            _   => continue
        }
    }

    bbs
}

pub fn parse_bbir_file(filename: &str) -> Program {
    let lines = read_lines(filename);

    let bbs = lex_bbir(lines);
    Program { bbs }
}

fn is_var(s: &str) -> bool {
    !s.parse::<i32>().is_ok()
}

const INTRINSIC_VARS: &[&str] = &[
    "$root", // Variable used to indicate if frame is active.
    "$isp",  // Block ID for block called if this ever returns. In META used for current block.
    "ret",  // Return position for internal calls. In META used for next block.
    "$tmp_copy", // TMP Variable for copy.
    "$tmp_a",    // TMP for first param.
    "$tmp_b",    // TMP for second param.
];

// Only set is allowed to have intrinsics.
fn compile_op(op: Operation, vars: &[String], current_position: &mut String) -> String {
    let pos_holder = RefCell::new(current_position);

    let get_index = |s: &str| -> isize {
        let idx = INTRINSIC_VARS.iter().position(|v| *v==s).unwrap_or_else(
            || vars.iter().position(|v| v==s).unwrap() + INTRINSIC_VARS.len()
        );
        idx as isize
    };

    let move_to = |var: &str| -> String {
        let delta: isize = get_index(var) - get_index(&pos_holder.borrow());
        **pos_holder.borrow_mut() = var.to_string();
        let dir = if delta > 0 { ">" } else { "<" }.to_string();
        dir.repeat(delta.unsigned_abs())
    };

    let move_val = |from: &str, to: &str| -> String {
        move_to(from) + "[-" + &move_to(to) + "+" + &move_to(from) + "]"
    };

    let zero = "[-]";

    let copy_to = |d: &str, s: &str| -> String {
        move_to(d) + zero + &move_to(s) + "[-" + &move_to("$tmp_copy") + "+" + &move_to(d) + "+" + &move_to(s) + "]"
        + &move_val("$tmp_copy", s)
    };

    match op.op_type {
        OpTy::Getc => move_to(&op.args[0]) + ",",
        OpTy::Putc => move_to(&op.args[0]) + ".",
        OpTy::Set => {
            if is_var(&op.args[1]) {
                copy_to(&op.args[0], &op.args[1])
            } else {
                let val = op.args[1].parse::<i32>().unwrap();
                let sign = if val >= 0 {"+"} else {"-"};
                move_to(&op.args[0]) + zero + &sign.repeat(val.unsigned_abs() as usize)
            }
        },
        OpTy::Add => {
            copy_to("$tmp_a", &op.args[1]) + &copy_to("$tmp_b", &op.args[2]) + &move_val("$tmp_b", "$tmp_a")
            + &move_to(&op.args[0]) + zero + &move_val("$tmp_a", &op.args[0])
        },
        OpTy::Sub => {
            copy_to("$tmp_a", &op.args[1]) + &copy_to("$tmp_b", &op.args[2])
            + &move_to("$tmp_b") + "[-" + &move_to("$tmp_a") + "-" + &move_to("$tmp_b") + "]"
            + &move_to(&op.args[0]) + zero + &move_val("$tmp_a", &op.args[0])
        },
        OpTy::IsZero => {
            move_to(&op.args[0]) + zero + "+" + &copy_to("$tmp_a", &op.args[1]) + &move_to("$tmp_a")
            + "[" + &move_to(&op.args[0]) + "-" + &move_to("$tmp_a") + zero + "]"
        },
        OpTy::Or => {
            copy_to("$tmp_a", &op.args[1]) + &copy_to("$tmp_b", &op.args[2]) + &move_val("$tmp_b", "$tmp_a")
            + &move_to(&op.args[0]) + zero + &move_to("$tmp_a")
            + "[" + &move_to(&op.args[0]) + "+" + &move_to("$tmp_a") + zero + "]"
        },
    }
}

fn compile_finalizer(fin: Finalizer, vars: &[String], current_position: &mut String, stackframe_size: usize, bb_indices: &HashMap<Label, usize>) -> String {
    let pos_holder = RefCell::new(current_position);

    let get_index = |s: &str| -> isize {
        dbg!(s, vars);
        let idx = INTRINSIC_VARS.iter().position(|v| *v==s).unwrap_or_else(
            || vars.iter().position(|v| v==s).unwrap() + INTRINSIC_VARS.len()
        );
        idx as isize
    };

    let move_to = |var: &str| -> String {
        let delta: isize = get_index(var) - get_index(&pos_holder.borrow());
        **pos_holder.borrow_mut() = var.to_string();
        let dir = if delta > 0 { ">" } else { "<" }.to_string();
        dir.repeat(delta.unsigned_abs())
    };

    let move_val = |from: &str, to: &str| -> String {
        move_to(from) + "[-" + &move_to(to) + "+" + &move_to(from) + "]"
    };

    let zero = "[-]";

    let copy_to = |d: &str, s: &str| -> String {
        move_to(d) + zero + &move_to(s) + "[-" + &move_to("$tmp_copy") + "+" + &move_to(d) + "+" + &move_to(s) + "]"
        + &move_val("$tmp_copy", s)
    };

    let stack_back  = "<".repeat(stackframe_size);
    let stack_front = ">".repeat(stackframe_size);

    let move_to_meta = || -> String { move_to("$root") + &stack_back + "[" + &stack_back + "]"};
    let move_to_current = || -> String { move_to("$root") + &stack_front + "[" + &stack_front + "]"};

    match fin {
        Finalizer::Branch { condition, positive, negative } => {
            let p_idx = bb_indices[&positive];
            let n_idx = bb_indices[&negative];

            let delta = if p_idx > n_idx {
                "+".repeat(p_idx-n_idx)
            } else {
                "-".repeat(n_idx-p_idx)
            };

            move_to_meta() + &move_to("ret") + &"+".repeat(n_idx) + &move_to_current()
            + &copy_to("$tmp_a", &condition) + &move_to("$tmp_a") + "[" + zero + &move_to_meta()
            + &move_to("ret") + &delta + &move_to_current() + &move_to("$tmp_a") + "]" + &move_to("$root")
        },
        Finalizer::Call { ret_label, call_label, args } => {
            todo!()
        },
        Finalizer::Return { ret_value } => {
            let start = move_to("$root");
            let mut ret_move = String::new();
            if let Some(ret) = ret_value {
                ret_move = stack_back.to_string() + &move_to("ret") + zero + &stack_front 
                + &move_to(&ret) + "[-" + &stack_back + &move_to("ret") + "+" + &stack_front + &move_to(&ret) + "]"
                + &move_to("$root");
            }
            start + &ret_move + &move_to("$isp")
            + "[-" + &move_to_meta() + &move_to("ret") + "+" + &move_to_current() + &move_to("$isp") + "]"
            + &move_to("$root") + &stack_back + "-"
        },
    }
}

// Compile bb assuming, current pointer is $root.
fn compile_bb(bb: BasicBlock, variables: &HashMap<String, Vec<String>>, stackframe_size: usize, bb_indices: &HashMap<Label, usize>) -> String {
    let mut current_position = "$root".to_string();
    let bf_ops: Vec<_> = bb.operations.into_iter().map(|op| {
        compile_op(op, &variables[&bb.label.family], &mut current_position)
    }).collect();

    let bf_final = compile_finalizer(bb.finalizer, &variables[&bb.label.family], &mut current_position, stackframe_size, bb_indices);

    bf_ops.join("") + &bf_final
}

fn bb_wrapper(stackframe_size: usize, initial_bb: usize) -> (String, String, String, String) {
    let current_position = "$root".to_string();
    let pos_holder = RefCell::new(current_position);

    let get_index = |s: &str| -> isize {
        let idx = INTRINSIC_VARS.iter().position(|v| *v==s).unwrap();
        idx as isize
    };

    let move_to = |var: &str| -> String {
        let delta: isize = get_index(var) - get_index(&pos_holder.borrow());
        *pos_holder.borrow_mut() = var.to_string();
        let dir = if delta > 0 { ">" } else { "<" }.to_string();
        dir.repeat(delta.unsigned_abs())
    };

    let move_val = |from: &str, to: &str| -> String {
        move_to(from) + "[-" + &move_to(to) + "+" + &move_to(from) + "]"
    };

    let zero = "[-]";

    let copy_to = |d: &str, s: &str| -> String {
        move_to(d) + zero + &move_to(s) + "[-" + &move_to("$tmp_copy") + "+" + &move_to(d) + "+" + &move_to(s) + "]"
        + &move_val("$tmp_copy", s)
    };

    let stack_back  = "<".repeat(stackframe_size);
    let stack_front = ">".repeat(stackframe_size);

    let move_to_meta = || -> String { stack_back.to_string() + "[" + &stack_back + "]"};
    let move_to_current = || -> String { stack_front.to_string() + "[" + &stack_front + "]"};

    let prior = move_to("$isp") + "-" + &copy_to("$tmp_a", "$isp")
    + &move_to("$tmp_b") + zero + "+"
    + &move_to("$tmp_a") + "[[-]" + &move_to("$tmp_b") + "-" + &move_to("$tmp_a") + "]"
    + &move_to("$tmp_b") + "[[-]" + &move_to("$root") + &move_to_current();
    let posterior = move_to_meta() + &move_to("$tmp_b") + "]" + &move_to("$root");

    let prog_prior = move_to("ret") + &"+".repeat(initial_bb) + "[" + &move_val("ret", "$isp") + &move_to("$root");
    let prog_post  = move_to("ret") + "]";

    (prior, posterior, prog_prior, prog_post)
}

pub fn compile_program(prog: Program) -> String {
    let families: HashSet<String> = prog.bbs.iter().map(|bb| bb.label.family.to_string()).collect();
    let variables: HashMap<String, Vec<String>> = families.into_iter().map(|family| {
        let mut vars: HashSet<String> = HashSet::new();
        let bbs = prog.bbs.iter().filter(|bb| bb.label.family == family);
        for bb in bbs {
            bb.params.iter().for_each(|p| {vars.insert(p.to_string());});

            for o in &bb.operations {
                o.args.iter().filter(|s| is_var(s)).for_each(|v| { vars.insert(v.to_string()); });
            }

            // We ignore finalizers, because they will always need an input.
        }

        (family, vars.into_iter().collect())
    }).collect();

    let max_vars = variables.values().map(|v| v.len()).max().unwrap();
    let stackframe_size = INTRINSIC_VARS.len() + max_vars;
    let bb_indices: HashMap<Label, usize> = prog.bbs.iter().enumerate().map(|(idx, bb)| (bb.label.clone(), idx+1)).collect();

    let entry = Label::try_from("main:in").unwrap();
    let (bb_prior, bb_post, prog_pre, prog_post) = bb_wrapper(stackframe_size, bb_indices[&entry]);

    let bf_bbs: Vec<String> = prog.bbs.into_iter().map(|bb| bb_prior.to_string() + &compile_bb(bb, &variables, stackframe_size, &bb_indices) + &bb_post).collect();

    prog_pre + &bf_bbs.join("") + &prog_post
}


