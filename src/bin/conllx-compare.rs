extern crate colored;
extern crate conllx;
extern crate conllx_utils;
extern crate getopts;

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::env::args;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::process;

use colored::*;
use conllx::Token;
use conllx_utils::{or_exit, LayerCallback, LAYER_CALLBACKS};
use getopts::Options;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] FILE...", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optopt(
        "l",
        "layer",
        "layer(s) to compare (form, lemma, cpos, pos, features, \
         head, headrel, phead, or pheadrel, default: headrel)",
        "LAYER[,LAYER]",
    );
    opts.optopt(
        "s",
        "show",
        "extra layer(s) to show from first file (form, lemma, cpos, \
         pos, features, head, headrel, phead, or pheadrel, default: form)",
        "LAYER[,LAYER]",
    );
    let matches = or_exit(opts.parse(&args[1..]));

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let callbacks = process_callbacks(matches.opt_str("l"), vec![&LAYER_CALLBACKS["headrel"]]);
    let show_callbacks = process_callbacks(matches.opt_str("s"), vec![&LAYER_CALLBACKS["form"]]);

    if matches.free.len() != 2 {
        print_usage(&program, opts);
        return;
    }

    let reader1 = or_exit(reader(&matches.free[0]));
    let reader2 = or_exit(reader(&matches.free[1]));

    or_exit(compare_sentences(
        reader1,
        reader2,
        &callbacks,
        &show_callbacks,
    ));
}

fn reader(filename: &str) -> io::Result<conllx::Reader<Box<BufRead>>> {
    let file = File::open(filename)?;
    let buf_read = Box::new(BufReader::new(file));
    Ok(conllx::Reader::new(buf_read))
}

fn process_callbacks(
    callback_option: Option<String>,
    default: Vec<&'static LayerCallback>,
) -> Vec<&'static LayerCallback> {
    if callback_option.is_none() {
        return default;
    }

    let mut callbacks = Vec::new();
    for layer_str in callback_option.unwrap().split(',') {
        match LAYER_CALLBACKS.get(layer_str) {
            Some(c) => callbacks.push(c),
            None => {
                println!("Unknown layer: {}", layer_str);
                process::exit(1)
            }
        }
    }

    callbacks
}

fn compare_sentences(
    reader1: conllx::Reader<Box<BufRead>>,
    reader2: conllx::Reader<Box<BufRead>>,
    diff_callbacks: &[&LayerCallback],
    show_callbacks: &[&LayerCallback],
) -> conllx::Result<()> {
    for (sent1, sent2) in reader1.into_iter().zip(reader2.into_iter()) {
        let (sent1, sent2) = (sent1?, sent2?);
        let tokens1 = sent1.as_tokens();
        let tokens2 = sent2.as_tokens();

        let diff = diff_indices(tokens1, tokens2, diff_callbacks)?;

        if !diff.is_empty() {
            print_diff(tokens1, tokens2, diff_callbacks, show_callbacks);
            println!();
        }
    }

    Result::Ok(())
}

fn print_diff(
    tokens1: &[Token],
    tokens2: &[Token],
    diff_callbacks: &[&LayerCallback],
    show_callbacks: &[&LayerCallback],
) {
    for idx in 0..tokens1.len() {
        let mut columns = Vec::new();

        for callback in show_callbacks {
            columns.push(
                callback(&tokens1[idx])
                    .unwrap_or(Cow::Borrowed("_"))
                    .into_owned(),
            );
        }

        for callback in diff_callbacks {
            let col1 = callback(&tokens1[idx]).unwrap_or(Cow::Borrowed("_"));
            let col2 = callback(&tokens2[idx]).unwrap_or(Cow::Borrowed("_"));

            if col1 != col2 {
                columns.push(format!("{}", col1.red()));
                columns.push(format!("{}", col2.red()));
            } else {
                columns.push(col1.into_owned());
                columns.push(col2.into_owned());
            }
        }

        println!("{}\t{}", idx + 1, columns.join("\t"));
    }
}

fn diff_indices(
    tokens1: &[Token],
    tokens2: &[Token],
    diff_callbacks: &[&LayerCallback],
) -> conllx::Result<BTreeSet<usize>> {
    if tokens1.len() != tokens2.len() {
        return Result::Err(
            format!(
                "Different number of tokens: {} {}",
                tokens1.len(),
                tokens2.len()
            ).into(),
        );
    }

    let mut indices = BTreeSet::new();

    'tokenloop: for i in 0..tokens1.len() {
        for layer_callback in diff_callbacks {
            if layer_callback(&tokens1[i]) != layer_callback(&tokens2[i]) {
                indices.insert(i);
                continue 'tokenloop;
            }
        }
    }

    Result::Ok(indices)
}
