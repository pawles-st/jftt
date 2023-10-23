use std::env;
use std::fs;

fn create_lps(pattern: &str) -> Vec<usize> {
    let mut lps = Vec::new();

    let pattern_chars: Vec<char> = pattern.chars().collect();
    /*let mut pattern_parts: Vec<&str> = Vec::new();
    for i in pattern.char_indices().map(|(idx, _)| idx) {
        pattern_parts.push(&pattern[..i]);
    }
    pattern_parts.push(&pattern[..]);*/
    lps.reserve(pattern_chars.len());

    let mut k = 0;
    lps.push(0);
    for q in 1..pattern_chars.len() {
        while k > 0 && pattern_chars[k] != pattern_chars[q] {
            k = lps[k - 1];
        }
        if pattern_chars[k] == pattern_chars[q] {
            k += 1;
        }
        lps.push(k);
    }

    return lps;
}

fn find(lps: &Vec<usize>, pattern: &str, text: &str) {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let mut text_chars = text.chars();

    let mut q = 0;
    for i in 0..text.chars().count() {
        let c = text_chars.next().unwrap();
        while q > 0 && pattern_chars[q] != c {
            q = lps[q - 1];
        }
        if pattern_chars[q] == c {
            q += 1;
        }
        if q == pattern_chars.len() {
            println!("{}", i + 1 - pattern_chars.len());
            q = lps[q - 1];
        }
    }
}

fn main() {
    let args : Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Invalid number of arguments");
        std::process::exit(1);
    }

    let pattern = &args[1];
    let filepath = &args[2];
    let text = fs::read_to_string(filepath).expect("can't read file");

    let lps = create_lps(&pattern);

    println!("lps table: {:?}", lps);

    find(&lps, &pattern, &text);
}
