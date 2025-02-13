
// PLECo II

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::process::{self, exit};
use std::sync::{Arc, Mutex};
use std::env::{self, args};
use crate::lexer;
use crate::buffer;

fn error(message: &str) {
    eprintln!("[ PLECo Error ]\t{}", message);
    process::exit(1);
}


pub struct PLECo {
    buffer: Arc<Mutex<buffer::ViewBuffer>>,
    vars: Arc<Mutex<HashMap<String, lexer::Token>>>,
}

impl PLECo {
    pub fn new() -> Self {
        let mut obj = Self {
            buffer: Arc::new(Mutex::new(buffer::ViewBuffer::new("tmp.txt"))),
            vars: Arc::new(Mutex::new(HashMap::new())),
        };

        {
            if let Some(home_dir) = std::env::home_dir() {
                obj.vars.lock().unwrap().insert(String::from("HOME"), lexer::Token::String(home_dir.to_str().unwrap().to_string()));
            }
        }

        obj
    }

    pub fn run(&self) {
        loop {
            let mut command = String::new();
            if io::stdin().read_line(&mut command).is_err() {
                println!("?");
                continue;
            }
            self.handle_command(command.trim());
        }
    }

    pub fn handle_command(&self, command: &str) {
        let commands = lexer::Lexer::new(command).tokenize();

        #[cfg(debug_assertions)]
        println!("{:?}", commands);

        let mut pc = 0;

        while pc < commands.len() {
            if let Some(com) = commands.get(pc) {
                match com {
                    lexer::Token::Command('a') => pc += self.cmd_insert(pc, &commands),
                    lexer::Token::Command('b') => self.buffer.lock().unwrap().cur_move_left(),
                    lexer::Token::Command('f') => self.buffer.lock().unwrap().cur_move_right(),
                    lexer::Token::Command('r') => self.buffer.lock().unwrap().remove_char(),
                    lexer::Token::Command('R') => {
                        let mut buffer = self.buffer.lock().unwrap();
                        buffer.buffer.clear();
                        buffer.cursor = 0;
                    }
                    lexer::Token::Command('v') => println!("{}", self.buffer.lock().unwrap().buffer),
                    lexer::Token::Command('q') => process::exit(0),
                    lexer::Token::Command('#') => break,
                    lexer::Token::Command('t') => pc += self.cmd_jump_cur(pc, &commands),
                    lexer::Token::Command('s') => pc += self.cmd_search(pc, &commands),
                    lexer::Token::Command('@') => pc += self.cmd_define(pc, &commands),
                    lexer::Token::Command('!') => pc += self.cmd_set_filename(pc, &commands),
                    lexer::Token::Command('S') => pc += self.cmd_save_file(pc, &commands),
                    lexer::Token::Command('x') => pc += self.cmd_load_file(pc, &commands),
                    lexer::Token::Command('=') => pc += self.cmd_equal(pc, &commands),
                    // lexer::Token::Command('>') => pc += self.cmd_lessthan(pc, &commands),
                    // lexer::Token::Command('<') => pc += self.cmd_morethan(pc, &commands),
                    lexer::Token::Command('M') => pc += self.cmd_execute_func(pc, &commands),
                    // lexer::Token::Command('J') => pc += self.cmd_add_str(pc, &commands),
                    lexer::Token::Command('l') => pc += self.cmd_import(pc, &commands),
                    lexer::Token::MultiLengthCommand(cmd) => pc += self.handle_multicommand(pc, &commands, cmd),
                    lexer::Token::Command(c) => {
                        if let Some(lexer::Token::Code(func_code)) = self.vars.lock().unwrap().get(&format!("{}", c)) {
                            self.handle_command(&func_code);
                        }
                    }
                    _ => {}
                }
            }
            pc += 1;
        }
    }

    fn handle_multicommand(&self, pc: usize, commands:  &[lexer::Token], cmd: &str) -> usize {

        if cmd == "LI" {
            return self.cmd_loop_inf(pc, commands);
        }

        if cmd == "Lo" {
            return self.cmd_loop_count(pc, commands);
        }

        if cmd == "IF" {
            return self.cmd_if_integer(pc, commands);

        } 

        0

    }


    fn cmd_loop_inf(&self, pc: usize, commands:  &[lexer::Token]) -> usize {
        if let Some(lexer::Token::Code(code)) = commands.get(pc+1) {
            loop {
                self.handle_command(code);
            }
            
            return 1

        }
        0
    }   

    fn cmd_if_integer(&self, pc: usize, commands:  &[lexer::Token]) -> usize {
        if let (Some(expr), Some(lexer::Token::Code(true_code)), Some(lexer::Token::Code(false_code))) = (commands.get(pc+1), commands.get(pc+2), commands.get(pc+3)) {

            let mut param_ = 0;
            if let lexer::Token::Integer(param) = expr {
                param_ = *param;
            }

            if let lexer::Token::Var(param) = expr {
                if let Some(param1) = self.vars.lock().unwrap().get(param) {
                    if let lexer::Token::Integer(param) = param1 {
                        param_ = *param;
                    }
                }
            }


            if let lexer::Token::Expr(param) = expr {
                if let Some(lexer::Token::Integer(param)) = self.process_expr(param) {
                    param_ = param
                } else {
                    error("formula eval error");
                }
            }

            if param_ > 0 {
                self.handle_command(true_code);
            } else {
                self.handle_command(false_code);
            }

            return 2
        }
        0
    }    

    fn cmd_loop_count(&self, pc: usize, commands:  &[lexer::Token]) -> usize {
        if let (Some(lexer::Token::Integer(count)), Some(lexer::Token::Code(code))) = (commands.get(pc+1), commands.get(pc+2)) {
            for _ in 0..*count {
                self.handle_command(code);
            }
            return 2;
        }
        0
    }    

    // バッファーにテキストを追加します
    fn cmd_insert(&self, pc: usize, commands:  &[lexer::Token]) -> usize {

        // 変数である場合と文字列リテラルである場合で処理を分けます

        if let Some(args1_token) = commands.get(pc+1) {
            #[cfg(debug_assertions)]
            println!("{:?}", args1_token);

            match args1_token.clone() {
                // 文字列リテラルである場合
                lexer::Token::String(string) => {
                    for c in string.chars() {
                        self.buffer.lock().unwrap().add_char(c);
                    }
                },
                // 変数である場合
                lexer::Token::Var(varname) => {
                    let vars = self.vars.lock().unwrap();

                    if let Some(value) = vars.get(&varname) {
                        if let lexer::Token::String(string) = value {
                            for c in string.chars() {
                                self.buffer.lock().unwrap().add_char(c);
                            }
                        } else if let lexer::Token::Integer(value) = value {
                            for c in value.to_string().chars() {
                                self.buffer.lock().unwrap().add_char(c);
                            }
                        } else {
                            error("type mismatch");
                        }
                    } else {
                        error("variables do not exist");
                    }
                },
                lexer::Token::Integer(value) => {
                    let string = value.to_string();
                    for c in string.chars() {
                        self.buffer.lock().unwrap().add_char(c);
                    }
                }
                _ => { error("type mismatch"); }
            }
            return 1;

        }

        0

    }

    // カーソルを指定の位置に移動させる
    fn cmd_jump_cur(&self, pc: usize, commands:  &[lexer::Token]) -> usize {

        // 変数である場合と数字リテラルである場合で処理を分けます

        if let Some(args1_token) = commands.get(pc+1) {
            let mut buffer = self.buffer.lock().unwrap();
            match args1_token.clone() {
                // 数字リテラルである場合
                lexer::Token::Integer(to) => if to >= 0 && to < buffer.buffer.len() as i32 {
                    buffer.cursor = to as usize
                },
                // 変数である場合
                lexer::Token::Var(varname) => {
                    let vars = self.vars.lock().unwrap();

                    if let Some(value) = vars.get(&varname) {
                        if let lexer::Token::Integer(to) = value {
                            if *to >= 0 && *to < buffer.buffer.len() as i32 {
                                buffer.cursor = *to as usize
                            }
                        } else {
                            error("type mismatch");
                        }
                    } else {
                        error("variables do not exist");
                    }
                },
                _ => { error("type mismatch"); }
            }
            return 1;

        }

        0

    }

    // 検索をかける
    fn cmd_search(&self, pc: usize, commands:  &[lexer::Token]) -> usize {
        // 変数である場合と文字列リテラルである場合で処理を分けます
        let mut buffer = self.buffer.lock().unwrap();

        if let Some(args1_token) = commands.get(pc+1) {
            match args1_token.clone() {
                // 文字列リテラルである場合
                lexer::Token::String(string) => {
                    if let Some(pos) = buffer.buffer.find(&string) {
                        buffer.cursor = pos;
                    }
                },
                // 変数である場合
                lexer::Token::Var(varname) => {
                    let vars = self.vars.lock().unwrap();

                    if let Some(value) = vars.get(&varname) {
                        if let lexer::Token::String(string) = value {
                            if let Some(pos) = buffer.buffer.find(string) {
                                buffer.cursor = pos;
                            }
                        } else {
                            error("type mismatch");
                        }
                    } else {
                        error("variables do not exist");
                    }
                },
                _ => { error("type mismatch"); }
            }
            return 1;

        }

        0

    }

    // 変数を定義
    fn cmd_define(&self, pc: usize, commands:  &[lexer::Token]) -> usize {
        // 変数を定義します
        let mut buffer = self.buffer.lock().unwrap();

        let mut vars = self.vars.lock().unwrap();

        if let (Some(args1_token), Some(args2_token)) = (commands.get(pc+1), commands.get(pc+2)) {

            let mut token_ = args2_token.clone();

            if let lexer::Token::Var(varname) = args1_token {

                if let lexer::Token::Var(var) = args2_token {
                    if let Some(token) = vars.get(var) {
                        token_ = token.clone();
                    }
                }

                if let lexer::Token::Expr(expr) = args2_token {
                    
                    if let Some(result) = self.process_expr(expr) {
                        token_ = result
                    } else {
                        error("formula eval error");
                    }

                }

                vars.insert(varname.clone(), token_);
            }

            return 2;
        }

        0

    }

    // ファイル名を定義する
    fn cmd_set_filename(&self, pc: usize, commands:  &[lexer::Token]) -> usize {
        // 変数である場合と文字列リテラルである場合で処理を分けます
        let mut buffer = self.buffer.lock().unwrap();

        if let Some(args1_token) = commands.get(pc+1) {
            match args1_token.clone() {
                // 文字列リテラルである場合
                lexer::Token::String(string) => {
                    buffer.filename = string;
                },
                // 変数である場合
                lexer::Token::Var(varname) => {
                    let vars = self.vars.lock().unwrap();

                    if let Some(value) = vars.get(&varname) {
                        if let lexer::Token::String(string) = value {
                            buffer.filename = string.clone()
                        } else {
                            error("type mismatch");
                        }
                    } else {
                        error("variables do not exist");
                    }
                },
                _ => { error("type mismatch"); }
            }
            return 1;

        }

        0

    }

    // ファイルを読み込む
    fn cmd_load_file(&self, pc: usize, commands:  &[lexer::Token]) -> usize {
        // 変数である場合と文字列リテラルである場合で処理を分けます
        let mut buffer = self.buffer.lock().unwrap();

        match fs::File::open(&buffer.filename) {
            Ok(mut file) => {
                file.read_to_string(&mut buffer.buffer);
            },
            Err(_) => { error("cannot read the file because it does not exist"); }
        }

        0

    }

    // ファイルを保存する
    fn cmd_save_file(&self, pc: usize, commands:  &[lexer::Token]) -> usize {
        // 変数である場合と文字列リテラルである場合で処理を分けます
        let mut buffer = self.buffer.lock().unwrap();

        match fs::File::create(&buffer.filename) {
            Ok(mut file) => {
                file.write_all(buffer.buffer.clone().as_bytes());
            },
            Err(_) => { error("file cannot be created."); }
        }

        0

    }


    // イコール
    fn cmd_equal(&self, pc: usize, commands:  &[lexer::Token]) -> usize {

        if let (Some(obj1), Some(obj2), Some(lexer::Token::Code(true_code)), Some(lexer::Token::Code(false_code))) = (commands.get(pc+1), commands.get(pc+2), commands.get(pc+3), commands.get(pc+4)) {
            if obj1 == obj2 {
                self.handle_command(true_code);
            } else {
                self.handle_command(false_code);
            }

            return 4
        }

        0
    }

    // マクロ実行
    fn cmd_execute_func(&self, pc: usize, commands:  &[lexer::Token]) -> usize {

        if let Some(lexer::Token::Code(code)) = commands.get(pc+1) {
            self.handle_command(code);
            return 1
        }

        0
    }

    // モジュールを読み込む
    fn cmd_import(&self, pc: usize, commands:  &[lexer::Token]) -> usize {

        if let Some(lexer::Token::String(fname)) = commands.get(pc+1) {
            
            match fs::File::open(fname) {
                Ok(mut file) => {
                    let mut buf = String::new();
                    file.read_to_string(&mut buf);
                    self.handle_command(&buf);
                },
                Err(_) => {
                    error("Module file does not exist");
                }
            }
            
            return 1
        }

        0
    }


    // コマンドとは関係ない

    // 数式計算

    fn process_expr(&self, expr: &str) -> Option<lexer::Token> {

        let tokens = lexer::Lexer::new(expr).tokenize();
        #[cfg(debug_assertions)]
        println!("{:?}", tokens);
        if let Some(operation) = tokens.get(0) {

            if *operation == lexer::Token::MultiLengthCommand(String::from("CT")) {
                if let Some(param) = tokens.get(1) {

                    let mut pat = "";
                    let vars = self.vars.lock().unwrap();

                    if let lexer::Token::Var(param) = param {
                        if let Some(param) = vars.get(param) {
                            if let lexer::Token::String(param) = param {
                                let pat_ = param;
                                pat = pat_;
                            }
                        }
                    }

                    if let lexer::Token::String(param) = param {
                        pat = param
                    }

                    let count = self.buffer.lock().unwrap().buffer.matches(pat).count();
                    return Some(lexer::Token::Integer(count as i32));
                } 
            } else {


                if let (Some(param1), Some(param2)) = (tokens.get(1), tokens.get(2)) {

                    let mut param1_ = 0;
                    let mut param2_ = 0;

                    if let lexer::Token::Integer(param1) = param1 {
                        param1_ = *param1;
                    }

                    if let lexer::Token::Integer(param2) = param2 {
                        param2_ = *param2;
                    }

                    if let lexer::Token::Var(param1) = param1 {
                        if let Some(param1) = self.vars.lock().unwrap().get(param1) {
                            if let lexer::Token::Integer(param1) = param1 {
                                param1_ = *param1;
                            }
                        }
                    }

                    if let lexer::Token::Var(param2) = param2 {
                        if let Some(param2) = self.vars.lock().unwrap().get(param2) {
                            if let lexer::Token::Integer(param2) = param2 {
                                param2_ = *param2;
                            } 
                        }
                    }

                    if let lexer::Token::Expr(param1) = param1 {
                        if let Some(lexer::Token::Integer(param1)) = self.process_expr(param1) {
                            param1_ = param1
                        } else {
                            error("formula eval error");
                        }
                    }

                    if let lexer::Token::Expr(param2) = param2 {
                        if let Some(lexer::Token::Integer(param2)) = self.process_expr(param2) {
                            param2_ = param2
                        } else {
                            error("formula eval error");
                        }
                    }

                    match operation {
                        lexer::Token::Command('+') => return Some(lexer::Token::Integer(param1_ + param2_)),
                        lexer::Token::Command('-') => return Some(lexer::Token::Integer(param1_ - param2_)),
                        lexer::Token::Command('x') => return Some(lexer::Token::Integer(param1_ * param2_)),
                        lexer::Token::Command('/') => return Some(lexer::Token::Integer(param1_ / param2_)),
                        lexer::Token::Command('%') => return Some(lexer::Token::Integer(param1_ % param2_)),
                        lexer::Token::Command('=') => return Some(lexer::Token::Integer( if param1_ == param2_ { 1 } else { 0 } )),
                        lexer::Token::Command('>') => return Some(lexer::Token::Integer( if param1_ > param2_ { 1 } else { 0 } )),
                        lexer::Token::Command('<') => return Some(lexer::Token::Integer( if param1_ < param2_ { 1 } else { 0 } )),
                        lexer::Token::Command('!') => return Some(lexer::Token::Integer( if param1_ != param2_ { 1 } else { 0 } )),
                        _ => { error("unknown operation"); }
                    }
                }
            }

        }

        None

    }

}
