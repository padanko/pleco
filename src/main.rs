use std::env;
use std::fs;
use std::io::Write;
use std::io::{self, Read};
use std::process;

mod buffer;

fn main() {
    let args: Vec<String> = env::args().collect();

    // 初期設定
    let mut buffer = buffer::ViewBuffer::new("tmp.txt");
    let mut copy = buffer::ViewBuffer::new("copy");
    let mut secondary_buffer = String::new();

    // ファイル名が指定されている場合、ファイルを開いて読み込む
    if let Some(filename) = args.get(1) {
        buffer.filename = filename.clone();
        if let Err(_) = fs::File::open(filename).and_then(|mut file| file.read_to_string(&mut buffer.buffer)) {
            println!("Could not open file");
        }
    }

    // コマンドモード管理フラグ
    let mut is_escape_mode = false;
    let mut is_string_mode = false;

    // メインループ
    loop {
        let mut command = String::new();
        if io::stdin().read_line(&mut command).is_err() {
            println!("Failed to read input");
            continue;
        }

        // 入力コマンドを処理
        let command = command.trim();
        let mut chars = command.chars();

        while let Some(c) = chars.next() {

            if is_string_mode {
                handle_string_mode(
                    c,
                    &mut buffer,
                    &mut is_escape_mode,
                    &mut is_string_mode,
                );
            } else {
                if handle_command(
                    c,
                    &mut buffer,
                    &mut copy,
                    &mut secondary_buffer,
                    &mut is_string_mode,
                ) {
                    break; // コマンドが終了信号の場合
                }
            }
        }
    }
}

// 文字列モードの処理
fn handle_string_mode(
    c: char,
    buffer: &mut buffer::ViewBuffer,
    is_escape_mode: &mut bool,
    is_string_mode: &mut bool,
) {
    match c {
        '\\' if !*is_escape_mode => *is_escape_mode = true,
        'n' if *is_escape_mode => {
            buffer.add_char('\n');
            *is_escape_mode = false;
        }
        '\\' if *is_escape_mode => {
            buffer.add_char('\\');
            *is_escape_mode = false;
        }
        ';' if *is_escape_mode => {
            buffer.add_char(';');
            *is_escape_mode = false;
        }
        ';' if !*is_escape_mode => *is_string_mode = false,
        _ => buffer.add_char(c),
    }
}

// 通常コマンドの処理
fn handle_command(
    c: char,
    buffer: &mut buffer::ViewBuffer,
    copy: &mut buffer::ViewBuffer,
    secondary_buffer: &mut String,
    is_string_mode: &mut bool,
) -> bool {
    match c {
        'a' => { *is_string_mode = true }, 
        'v' => println!("{}", buffer.buffer),
        'V' => print!("{}", buffer.buffer),
        'b' => buffer.cur_move_left(),
        'f' => buffer.cur_move_right(),
        'r' => buffer.remove_char(),
        'R' => {
            buffer.buffer.clear();
            buffer.cursor = 0;
        }
        'q' => process::exit(0), // プログラム終了
        'l' => *secondary_buffer = buffer.buffer.len().to_string(),
        'o' => {
            for c in secondary_buffer.chars() {
                buffer.add_char(c);
            }
            secondary_buffer.clear();
        }
        'i' => *secondary_buffer = buffer.buffer.clone(),
        's' => {
            if let Some(pos) = buffer.buffer.find(&secondary_buffer.to_string()) {
                buffer.cursor = pos;
            }
        }
        '#' => return true, // コマンド終了
        'F' => buffer.cursor = 0,
        'L' => buffer.cursor = buffer.buffer.len(),
        'S' => {

            if let Ok(mut file) = fs::File::create(&buffer.filename) {
                let _ = file.write_all(buffer.buffer.as_bytes());
            }
        }
        '!' => {
            buffer.filename = (&secondary_buffer).to_string();
        }
        '>' => {
            copy.buffer = buffer.buffer.clone();
            copy.cur_move_right();
        }
        '<' => {
            copy.buffer = buffer.buffer.clone();
            copy.cur_move_left();
        }
        'c' => {
            copy.buffer = buffer.buffer.clone();
            if let Some(cpbuf) = buffer.buffer.get(buffer.cursor..(copy.cursor)) {
                copy.buffer = cpbuf.into();
            }
        }

        'p' => {
            for c in copy.buffer.chars().collect::<Vec<char>>() {
                buffer.add_char(c);
            }
        }
        _ => (),
    }
    false
}
