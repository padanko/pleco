use std::fs;
use std::io::{self, Read, Write};
use std::env;

mod buffer;
mod lexer;
mod pleco;

fn main() {

    let args: Vec<String> = env::args().collect();
    let pleco = pleco::PLECo::new();

    if let Some(fpath) = args.get(1) {
        match fs::File::open(fpath) {
            Ok(mut file) => {
                let mut plecorc = String::new();
                let _ = file.read_to_string(&mut plecorc);
                pleco.handle_command(&plecorc);
            }, 
            Err(_) => { }
        }

    }
    else {

        if let Some(plecorc) = env::home_dir() {
            match fs::File::open(format!("{}/.plecorc", plecorc.to_str().unwrap_or("~"))) {
                Ok(mut file) => {
                    let mut plecorc = String::new();
                    let _ = file.read_to_string(&mut plecorc);
                    pleco.handle_command(&plecorc);
                }, 
                Err(_) => { }
            }
        }



        pleco.run();
    }
}
