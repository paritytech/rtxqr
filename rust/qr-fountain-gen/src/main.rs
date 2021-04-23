use std::env;
use std::process;

use qr_fountain_gen::*;

fn main() {
    let source = Entry::new(env::args()).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });
    match &source.setupname {
        Some(a) => println!("Encoding data from file {} using setup file {}", &source.filename, a),
        None => println!("Encoding data from file {} using setup file default_constants", &source.filename),
    }
    let out_name = format!("{}.png", &source.filename);
    
    if let Err(e) = run_hex(&source, &out_name) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
    
}
