extern crate conllx;
extern crate conllx_utils;
extern crate getopts;
extern crate regex;
extern crate stdinout;

use std::collections::HashSet;
use std::env::args;
use std::io::BufWriter;
use std::process;

use conllx::{Features, Token, WriteSentence};
use conllx_utils::{layer_callback, or_exit, LayerCallback};
use getopts::Options;
use regex::Regex;
use stdinout::{Input, Output};

fn print_usage(program: &str, opts: Options) {
    let brief = format!(
        "Usage: {} [options] EXPR [INPUT_FILE] [OUTPUT_FILE]",
        program
    );
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
        "layer: form, lemma, cpos, pos, headrel, or pheadrel (default: form)",
        "LAYER",
    );
    opts.optopt(
        "m",
        "mark",
        "mark maching nodes using the given feature",
        "FEATURE",
    );
    let matches = or_exit(opts.parse(&args[1..]));

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let mark_feature = matches.opt_str("m");

    let callback = matches
        .opt_str("l")
        .as_ref()
        .map(|layer| match layer_callback(layer.as_str()) {
            Some(c) => c,
            None => {
                println!("Unknown layer: {}", layer);
                process::exit(1)
            }
        })
        .unwrap_or_else(|| layer_callback("form").unwrap());

    if matches.free.is_empty() || matches.free.len() > 3 {
        print_usage(&program, opts);
        return;
    }

    let re = or_exit(Regex::new(&matches.free[0]));
    let input = Input::from(matches.free.get(1));
    let reader = conllx::Reader::new(or_exit(input.buf_read()));

    let output = Output::from(matches.free.get(2));
    let mut writer = conllx::Writer::new(BufWriter::new(or_exit(output.write())));
    for sentence in reader {
        let mut sentence = or_exit(sentence);

        let matches = match_indexes(&re, &callback, &sentence);
        if matches.is_empty() {
            continue;
        }

        if let Some(ref feature) = mark_feature {
            for idx in matches {
                let mut features = sentence[idx]
                    .features()
                    .map(|f| f.as_map().clone())
                    .unwrap_or_default();
                features.insert(feature.clone(), None);
                sentence[idx].set_features(Some(Features::from_iter(features)));
            }
        }

        or_exit(writer.write_sentence(&sentence))
    }
}

fn match_indexes(re: &Regex, callback: &LayerCallback, sentence: &[Token]) -> HashSet<usize> {
    let mut indexes = HashSet::new();

    for (idx, token) in sentence.iter().enumerate() {
        if let Some(token) = callback(token) {
            if re.is_match(token.as_ref()) {
                indexes.insert(idx);
            }
        }
    }

    indexes
}
