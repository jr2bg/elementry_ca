// todo:
// 1. vector of len X of type bool
// 2. transition function
// 3. convert to concurrent

use rand;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use rayon::prelude::*;
use clap::{ArgGroup, Parser};

/// Program to simulate the evolution of a given elementary cellular automata
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(group(
            ArgGroup::new("exec")
                .required(true)
                .args(&[ "seq", "tp", "con-iter"]),
        ))]
struct Args {
    /// Number between 0-255 (inclusive) corresponding to the elementary cellular automata
    #[clap(short, long, default_value_t=110)]
    rule: usize,

    /// Number of evolutions
    #[clap(short, long, default_value_t=100)]
    n_transitions: usize,

    /// Size of the universe
    #[clap(short, long, default_value_t=50)]
    s_univ: usize,

    /*/// Type of execution
    #[clap(short, long, value_name = "EXEC", default_value="seq")]
    execution: String,*/

    /// Secuential execution
    #[clap(long)]
    seq: bool,

    /// Concurrent execution: thread pool
    #[clap(long)]
    tp: bool,

    /// Concurrent execution: concurrent iterator
    #[clap(long)]
    con_iter: bool,
}


pub struct Automata {
    pub size: usize,
    pub pre_univ: Vec<bool>,
    pub pos_univ: Vec<bool>,

}

fn num2bools(x: usize) -> Vec<bool>{
    // there are only 255 rules
    if x > 255 {
        panic!("Number must be at most 255");
    }

    // convert binary to collection of vecs for creating the rules
    let x_b = format!("{x:b}");
    let closure_bin_bool = |i| {
        match i {
            '0' => false,
            '1' => true,
            _ => panic!("Number different to 0 or 1"),
        }
    };
    let mut res: Vec<bool> = x_b.chars().map(closure_bin_bool).collect();

    // the vec must have 8 elements
    while res.len() < 8 {
        res.insert(0, false);
    }
    res
}


// function to create a closure containing the transition function given rule
fn create_fn_rules(res_trans: Vec<bool>) -> impl Fn((bool, bool, bool)) -> bool {
    move |neigh| {
        match neigh {
            (true, true, true) => res_trans[0],
            (true, true, false) => res_trans[1],
            (true, false, true) => res_trans[2],
            (true, false, false) => res_trans[3],
            (false, true, true) => res_trans[4],
            (false, true, false) => res_trans[5],
            (false, false, true) => res_trans[6],
            (false, false, false) => res_trans[7],
            _ => panic!("Not a tuple of three booleans"),
        }
    }
}

fn get_neigh(pos:usize, s_univ: usize, univ: &Vec<bool>) -> (bool, bool, bool) {
    match pos {
      x if x == 0 => (univ[s_univ - 1], univ[0], univ[1]),
      x if x == s_univ - 1 => (univ[s_univ -2], univ[s_univ -1], univ[0]),
      x => (univ[x - 1], univ[x], univ[x + 1]),
    }
}

fn rule30(neigh: (bool, bool, bool)) -> bool {
    match neigh {
        (false, false, false) => false,
        (false, false, true) => true,
        (false, true, false) => true,
        (false, true, true) => true,
        (true, false, false) => true,
        (true, false, true) => true,
        (true, true, false) => false,
        (true, true, true) => false,
    }
}

fn rule_pos(pos: usize, neigh:(bool, bool, bool)) -> (usize, bool) {
    (pos, rule30(neigh))
}

fn transition(s_univ:usize, univ: &Vec<bool>) -> Vec<bool> {
    let mut n_univ = Vec::with_capacity(s_univ);
    for i in 0..s_univ {
        n_univ.push(rule30(get_neigh(i, s_univ, univ)));
    }
    n_univ
}

fn sec_transition_closure<F>(
    s_univ:usize,
    univ:&Vec<bool>,
    rule:F
) -> Vec<bool> where
F: Fn((bool, bool, bool)) -> bool
{
    let mut n_univ = Vec::with_capacity(s_univ);
    for i in 0..s_univ {
        n_univ.push(rule(get_neigh(i, s_univ, univ)));
    }
    n_univ
}

fn conc_iter_trans(s_univ:usize, univ: Vec<bool>) -> Vec<bool> {
    let ret:Vec<bool> = (0..s_univ)
        .into_par_iter()
        .map(|indx| get_neigh(indx, s_univ, &univ))
        .map(|neigh| rule30(neigh))
        .collect();
    ret
}

// Make the closure thread safe
fn conc_iter_trans_clos<F>(
    s_univ:usize,
    univ: Vec<bool>,
    rule:F
) -> Vec<bool> where
F: Fn((bool, bool, bool)) -> bool + std::marker::Sync
{
    let ret:Vec<bool> = (0..s_univ)
        .into_par_iter()
        .map(|indx| get_neigh(indx, s_univ, &univ))
        .map(|neigh| rule(neigh))
        .collect();
    ret
}

fn _concurrent_transition(s_univ:usize, univ: Vec<bool>) -> Vec<bool> {
    let mut n_univ:Vec<bool> = vec![true; s_univ];
    let _n_threads = 2;
    let (tx_1, rx) = mpsc::channel();
    let tx_2 = tx_1.clone();
    let a_univ = Arc::new(univ);

    let a_univ_1 = Arc::clone(&a_univ);
    thread::spawn(move || {
        for i in 0..s_univ/2{
            tx_1.send(rule_pos(i, get_neigh(i, s_univ, &a_univ_1))).unwrap();
        }
    });

    let a_univ_2 = Arc::clone(&a_univ);
    thread::spawn(move || {
        for i in s_univ/2 .. s_univ {
            tx_2.send(rule_pos(i, get_neigh(i, s_univ, &a_univ_2))).unwrap();
        }
    });


    for received in rx {
        n_univ[received.0] = received.1;
    }
    n_univ
}


fn _main_trans_diff() {
    let s_univ: usize = 100;
    let n_transitions: usize = 1_000_000;
    let mut univ:Vec<bool> = (0..s_univ).map(|_| rand::random::<bool>()).collect();
    let convrt = |x| {
        if x { "X" } else { " " }
    };

    // println!("{:?}", univ.iter().map(|x| convrt(*x)).collect::<Vec<&str>>().join(""));
    // let pool = rayon::ThreadPoolBuilder::new().num_threads(5).build().unwrap();

    for _i in 0..n_transitions {
        // univ = transition(s_univ, &univ);
        // univ = concurrent_transition(s_univ, univ);
        //
        //univ = pool.install(|| transition(s_univ, &univ));
        univ = conc_iter_trans(s_univ, univ);

        //println!("{:?}", univ.iter().map(|x| convrt(*x)).collect::<Vec<&str>>().join(""));
    }
}

fn _main() {
    let s_univ: usize = 120;
    let n_transitions: usize = 40;
    let mut univ:Vec<bool> = (0..s_univ).map(|_| rand::random::<bool>()).collect();
    let convrt = |x| {
        if x { "X" } else { " " }
    };

    let rule = 153;
    let rule_v = num2bools(rule);
    let rule = create_fn_rules(rule_v);

    for i in 0..n_transitions {
        univ = sec_transition_closure(s_univ, &univ, &rule);
        println!("{:?}", univ.iter().map(|x| convrt(*x)).collect::<Vec<&str>>().join(""));
    }
}

fn main() {
    let args = Args::parse();
    // println!("args: {:?}", args);

    //let exec = args.execution;

    let s_univ = args.s_univ;
    let n_transitions = args.n_transitions;
    //let exec = args.execution;

    let mut univ:Vec<bool> = (0..s_univ).map(|_| rand::random::<bool>()).collect();
    let convrt = |x| {
        if x { "🟧" } else { "🟦" }
    };

    let rule_v = num2bools(args.rule);
    let rule = create_fn_rules(rule_v);

    //if exec == "tp" {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(5).build().unwrap();
    //}

    for i in 0..n_transitions {

        /*match exec.as_str() {
            "seq" => univ = sec_transition_closure(s_univ, &univ, &rule),
            "con_iter" => univ = conc_iter_trans_clos(s_univ, univ, &rule),
            "tp" => univ = pool.install(|| sec_transition_closure(s_univ, &univ, &rule)),
            _ => panic!("Not recognized execution type"),
        }*/

        if args.seq {
            univ = sec_transition_closure(s_univ, &univ, &rule);
        } else if args.tp {
            univ = pool.install(|| sec_transition_closure(s_univ, &univ, &rule));
        } else if args.con_iter {
            univ = conc_iter_trans_clos(s_univ, univ, &rule);
        } else {
            panic!("Not recognized execution type");
        }

        println!("{}", univ.iter().map(|x| convrt(*x)).collect::<Vec<&str>>().join(""));
    }
}
