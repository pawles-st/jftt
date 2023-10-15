use std::fs;
use std::env;
use std::collections::HashMap;

// create the transition table for the matcher automaton
fn create_transitions(pattern: &str) -> HashMap<(usize, char), usize> {
    let mut transition_table = HashMap::new();

    let mut pattern_parts: Vec<&str> = Vec::new();
    for i in pattern.char_indices().map(|(idx, _)| idx) {
        pattern_parts.push(&pattern[..i]);
    }
    pattern_parts.push(&pattern[..]);
    //println!("{:?}", pattern_parts);

    //let mut matched = String::new();
    //let mut pattern_it = pattern.chars();
    for q in 0..pattern_parts.len() {
        //println!("state {}", q);
        for c in pattern.chars() {
            if !transition_table.contains_key(&(q, c)) {
                //println!("input {}", c);
                let mut read = pattern_parts[q].to_owned();
                read.push(c);
                //println!("read {}", read);
                let mut read_it = read.char_indices().map(|(idx, _)| idx);
                //println!("{:?}", read_it);
                let mut k = usize::min(pattern_parts.len() - 1, q + 1);
                if q + 1 == pattern_parts.len() {
                    read_it.next();
                }
                while k > 0 && pattern_parts[k] != &read[read_it.next().unwrap()..] {
                    //println!("k = {}", k);
                    k -= 1;
                }
                transition_table.insert((q, c), k);
            }
        }

        /*
        if q != pattern.len() {
            matched.push(pattern_it.next().unwrap());
            println!("{}", matched);
        }
        */
    }

    return transition_table;
}

fn find(transition_table: HashMap<(usize, char), usize>, pattern_len: usize, text: &str) {
    let mut text_chars = text.chars();
    let mut q = 0;
    for i in 0..text.chars().count() {
        let c = text_chars.next().unwrap();
        if transition_table.contains_key(&(q, c)) {
            q = *transition_table.get(&(q, c)).unwrap();
        } else {
            q = 0;
        }
        

        if q == pattern_len {
            println!("{}", i + 1 - pattern_len);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Invalid number of arguments");
        std::process::exit(1);
    }

    let pattern = &args[1];
    //let pattern = "◆◆◆";
    let filepath = &args[2];
    //let filepath = "◆◆◆◆aabaa◆◆◆a";
    let text = fs::read_to_string(filepath).expect("can't read file");
    
    let transition_table = create_transitions(&pattern);

    println!("transition table: {:?}", transition_table);

    find(transition_table, pattern.chars().count(), &text);
}
