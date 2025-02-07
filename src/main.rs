// PLECo II

use std::collections::HashMap;
use std::env;
use std::env::var;
use std::fmt::format;
use std::fs;
use std::io::Write;
use std::io::{self, Read};
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
mod buffer;
mod lexer;

fn main() {
    let buffer = Arc::new(Mutex::new(buffer::ViewBuffer::new("tmp.txt")));
    let copy = Arc::new(Mutex::new(buffer::ViewBuffer::new("copy")));
    let secondary_buffer = Arc::new(Mutex::new(String::new()));
    let vars: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    let vars_i: Arc<Mutex<HashMap<String, i32>>> = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let mut command = String::new();
        if io::stdin().read_line(&mut command).is_err() {
            println!("?");
            continue;
        }

        let command = command.trim();

        handle_command(command, &buffer, &copy, &secondary_buffer, &vars, &vars_i);
    }
}

fn handle_command(
    command: &str,
    buffer: &Arc<Mutex<buffer::ViewBuffer>>,
    copy: &Arc<Mutex<buffer::ViewBuffer>>,
    secondary_buffer: &Arc<Mutex<String>>,
    vars: &Arc<Mutex<HashMap<String, String>>>,
    vars_i: &Arc<Mutex<HashMap<String, i32>>>,
) {
    let commands = lexer::Lexer::new(command).tokenize();
    let mut pc = 0;
    #[cfg(debug_assertions)]
    println!("COMMANDS {:?}", commands);
    while pc < commands.len() {
        if let Some(com) = (&commands).get(pc) {
            #[cfg(debug_assertions)]
            {
                if let lexer::Token::Command(_) = com {
                    println!("RUN {} {:?}", pc, com);
                }
            }

            match com {
                &lexer::Token::Command('a') => {
                    if pc + 1 != commands.len() {
                        let args = commands[pc + 1].clone();
                        let add_m = lexer::extract_var(args).unwrap_or(String::new());
                        if let Some(add_m) = vars.lock().unwrap().get(&add_m) {
                            let mut buffer = buffer.lock().unwrap();

                            for c in add_m.chars() {
                                buffer.add_char(c);
                            }
                        }
                        pc += 1;
                    }
                }
                &lexer::Token::Command('b') => buffer.lock().unwrap().cur_move_left(),
                &lexer::Token::Command('f') => buffer.lock().unwrap().cur_move_right(),
                &lexer::Token::Command('r') => buffer.lock().unwrap().remove_char(),
                &lexer::Token::Command('R') => {
                    let mut buffer_lock = buffer.lock().unwrap();
                    buffer_lock.buffer = String::new();
                    buffer_lock.cursor = 0;
                }
                &lexer::Token::Command('v') => println!("{}", buffer.lock().unwrap().buffer),
                &lexer::Token::Command('q') => process::exit(0),
                &lexer::Token::Command('#') => break,
                &lexer::Token::Command('s') => {
                    if pc + 1 != commands.len() {
                        let args = commands[pc + 1].clone();
                        let search_pat = lexer::extract_var(args).unwrap_or(String::new());
                        if let Some(search_pat) = vars.lock().unwrap().get(&search_pat) {
                            let mut buffer = buffer.lock().unwrap();

                            if let Some(cur_pos) = buffer.buffer.find(search_pat) {
                                buffer.cursor = cur_pos;
                            }
                        }

                        pc += 1;
                    }
                }

                &lexer::Token::Command('@') => {
                    if pc + 1 != commands.len() && pc + 2 != commands.len() {
                        let varname = commands[pc + 1].clone();
                        let varname = lexer::extract_var(varname).unwrap_or(String::new());
                        let text = commands[pc + 2].clone();
                        let text = lexer::extract_argument(text).unwrap_or(String::new());
                        vars.lock().unwrap().insert(varname, text);
                        pc += 2;
                    }
                }

                &lexer::Token::Command('!') => {
                    if pc + 1 != commands.len() {
                        let filename = commands[pc + 1].clone();
                        let filename = lexer::extract_var(filename).unwrap_or(String::new());
                        if let Some(filename) = vars.lock().unwrap().get(&filename) {
                            let mut buffer = buffer.lock().unwrap();

                            buffer.filename = filename.to_string();
                        }
                        pc += 1;
                    }
                }

                &lexer::Token::Command('S') => {
                    let buffer_lock = buffer.lock().unwrap();
                    if let Ok(mut file) = fs::File::create(&buffer_lock.filename) {
                        let _ = file.write_all(buffer_lock.buffer.as_bytes());
                    }
                }
                &lexer::Token::Command('x') => {
                    let mut buffer_lock = buffer.lock().unwrap();
                    if fs::File::open(&buffer_lock.filename)
                        .and_then(|mut file| file.read_to_string(&mut buffer_lock.buffer))
                        .is_err()
                    {
                        println!("?");
                    }
                }
                &lexer::Token::Command('V') => {
                    if pc + 1 != commands.len() {
                        let args = commands[pc + 1].clone();
                        let varname = lexer::extract_var(args).unwrap_or(String::new());
                        if let Some(text) = vars.lock().unwrap().get(&varname) {
                            println!("{}", text);
                        }
                        pc += 1;
                    }
                }
                _ => {}
            }

            if com == &lexer::Token::MultiLengthCommand("I@".to_string()) {
                if pc + 1 != commands.len() && pc + 2 != commands.len() {
                    let varname = commands[pc + 1].clone();
                    let varname = lexer::extract_var(varname).unwrap_or(String::new());
                    let int = commands[pc + 2].clone();
                    let int = lexer::extract_integer(int).unwrap_or(0);
                    vars_i.lock().unwrap().insert(varname, int);
                    pc += 2;
                }
            }
            if com == &lexer::Token::MultiLengthCommand("I@v".to_string()) {
                if pc + 1 < commands.len() && pc + 2 < commands.len() {
                    let varname = commands[pc + 1].clone();
                    let varname = lexer::extract_var(varname).unwrap_or(String::new());
                    let varname_ = commands[pc + 2].clone();
                    let varname_ = lexer::extract_var(varname_).unwrap_or(String::new());

                    // まず不変で参照を取得
                    let mut vars_i_ = vars_i.lock().unwrap();
                    let mut vars_i = vars_i_.clone();

                    let target = vars_i.get(&varname_).unwrap_or(&0); // デフォルトは "0"

                    // 次に可変で借用
                    vars_i_.insert(varname, target.clone());
                    pc += 2;
                }
            }
            if com == &lexer::Token::MultiLengthCommand("IA".to_string()) {
                if pc + 1 < commands.len() && pc + 2 < commands.len() {
                    let var_name_a =
                        lexer::extract_var(commands[pc + 1].clone()).unwrap_or(String::new());
                    let var_name_b =
                        lexer::extract_var(commands[pc + 2].clone()).unwrap_or(String::new());

                    let mut result = 0;

                    {
                        let vars_i = vars_i.lock().unwrap();
                        let a = vars_i.get(&var_name_a).unwrap_or(&0);
                        let b = vars_i.get(&var_name_b).unwrap_or(&0);

                        result = a + b;
                        #[cfg(debug_assertions)]
                        println!("RESULT {}", result)
                    }

                    let mut vars_i = vars_i.lock().unwrap();
                    vars_i.insert(var_name_a, result);

                    pc += 2;
                }
            }
            if com == &lexer::Token::MultiLengthCommand("IS".to_string()) {
                if pc + 1 < commands.len() && pc + 2 < commands.len() {
                    let var_name_a =
                        lexer::extract_var(commands[pc + 1].clone()).unwrap_or(String::new());
                    let var_name_b =
                        lexer::extract_var(commands[pc + 2].clone()).unwrap_or(String::new());

                    let mut result = 0;

                    {
                        let vars_i = vars_i.lock().unwrap();
                        let a = vars_i.get(&var_name_a).unwrap_or(&0);
                        let b = vars_i.get(&var_name_b).unwrap_or(&0);

                        result = a - b;
                        #[cfg(debug_assertions)]
                        println!("RESULT {}", result)
                    }

                    let mut vars_i = vars_i.lock().unwrap();
                    vars_i.insert(var_name_a, result);

                    pc += 2;
                }
            }
            if com == &lexer::Token::MultiLengthCommand("IM".to_string()) {
                if pc + 1 < commands.len() && pc + 2 < commands.len() {
                    let var_name_a =
                        lexer::extract_var(commands[pc + 1].clone()).unwrap_or(String::new());
                    let var_name_b =
                        lexer::extract_var(commands[pc + 2].clone()).unwrap_or(String::new());

                    let mut result = 0;

                    {
                        let vars_i = vars_i.lock().unwrap();
                        let a = vars_i.get(&var_name_a).unwrap_or(&0);
                        let b = vars_i.get(&var_name_b).unwrap_or(&0);

                        result = a * b;
                        #[cfg(debug_assertions)]
                        println!("RESULT {}", result)
                    }

                    let mut vars_i = vars_i.lock().unwrap();
                    vars_i.insert(var_name_a, result);

                    pc += 2;
                }
            }
            if com == &lexer::Token::MultiLengthCommand("ID".to_string()) {
                if pc + 1 < commands.len() && pc + 2 < commands.len() {
                    let var_name_a =
                        lexer::extract_var(commands[pc + 1].clone()).unwrap_or(String::new());
                    let var_name_b =
                        lexer::extract_var(commands[pc + 2].clone()).unwrap_or(String::new());

                    let mut result = 0;

                    {
                        let vars_i = vars_i.lock().unwrap();
                        let a = vars_i.get(&var_name_a).unwrap_or(&0);
                        let b = vars_i.get(&var_name_b).unwrap_or(&0);

                        result = a / b;
                        #[cfg(debug_assertions)]
                        println!("RESULT {}", result)
                    }

                    let mut vars_i = vars_i.lock().unwrap();
                    vars_i.insert(var_name_a, result);

                    pc += 2;
                }
            }

            if com == &lexer::Token::MultiLengthCommand("Io".to_string()) {
                if pc + 1 != commands.len() {
                    let a = commands[pc + 1].clone();
                    let a = lexer::extract_var(a).unwrap_or(String::new());

                    let vars_i = vars_i.lock().unwrap();

                    let av = vars_i.get(&a).unwrap_or(&0);
                    let av_s = av.to_string();

                    for c in av_s.chars() {
                        buffer.lock().unwrap().add_char(c);
                    }

                    pc += 1;
                }
            }
            if com == &lexer::Token::MultiLengthCommand("LI".to_string()) {
                if pc + 1 != commands.len() {
                    let a = commands[pc + 1].clone();
                    if let lexer::Token::Code(code) = a {
                        loop {
                            handle_command(&code, buffer, copy, secondary_buffer, vars, vars_i);
                        }
                    }
                }

                pc += 1
            }
            if com == &lexer::Token::MultiLengthCommand("Lo".to_string()) {
                if pc + 1 != commands.len() && pc + 2 != commands.len() {
                    let a = commands[pc + 1].clone();
                    let b: lexer::Token = commands[pc + 2].clone();
                    if let lexer::Token::Integer(count) = a {
                        if let lexer::Token::Code(code) = b {
                            for _ in 0..count {
                                handle_command(&code, buffer, copy, secondary_buffer, vars, vars_i);
                            }
                        }
                    }
                }

                pc += 2;
            }
        }
        pc += 1;
        #[cfg(debug_assertions)]
        thread::sleep(time::Duration::from_millis(100));
    }
}
