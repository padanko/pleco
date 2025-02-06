use std::fs;
use std::env;
use std::io::Write;
use std::io::{self, Read};
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
mod buffer;

fn main() {
    let buffer = Arc::new(Mutex::new(buffer::ViewBuffer::new("tmp.txt")));
    let copy = Arc::new(Mutex::new(buffer::ViewBuffer::new("copy")));
    let secondary_buffer = Arc::new(Mutex::new(String::new()));

    let is_escape_mode = Arc::new(Mutex::new(false));
    let is_string_mode = Arc::new(Mutex::new(false));

    let buffers = Arc::new(Mutex::new(vec!["".into(), "".into(), "".into(), "".into(), "".into()]));

    let buffers_cur = Arc::new(Mutex::new(0));

    let loop_count_string = Arc::new(Mutex::new(String::new()));

    let args: Vec<String> = env::args().collect::<Vec<String>>();

    if args.get(1).unwrap_or(&String::new()) == "" {
        loop {
            let mut command = String::new();
            if io::stdin().read_line(&mut command).is_err() {
                println!("?");
                continue;
            }

            let command = command.trim();

            handle_char(command, &buffer, &is_escape_mode, &is_string_mode, 
                &copy, &secondary_buffer, &buffers_cur, &buffers,
                &loop_count_string);

        }
    } else {
        handle_char(args.get(1).unwrap_or(&String::new()), &buffer, &is_escape_mode, &is_string_mode, 
            &copy, &secondary_buffer, &buffers_cur, &buffers,
            &loop_count_string);
    }
}


fn handle_char(
    command: &str,
    buffer: &Arc<Mutex<buffer::ViewBuffer>>,
    is_escape_mode: &Arc<Mutex<bool>>,
    is_string_mode: &Arc<Mutex<bool>>,
    copy: &Arc<Mutex<buffer::ViewBuffer>>,
    secondary_buffer: &Arc<Mutex<String>>,
    buffers_cursor: &Arc<Mutex<usize>>,
    buffers: &Arc<Mutex<Vec<String>>>,
    loop_count_string: &Arc<Mutex<String>>
) {
    let mut chars = command.chars();
    while let Some(c) = chars.next() {
        let mut is_string_mode_lock = is_string_mode.lock().unwrap();
        let mut is_escape_mode_lock = is_escape_mode.lock().unwrap();

        if *is_string_mode_lock {
            handle_string_mode(
                c,
                &buffer,
                &mut is_escape_mode_lock,
                &mut is_string_mode_lock,
            );
        } else {
            let mut loop_count_string_lock = loop_count_string.lock().unwrap();
            if c.is_ascii_digit() {
                loop_count_string_lock.push(c);
            } else {
                let loop_count = loop_count_string_lock.parse::<usize>().unwrap_or(1);
                loop_count_string_lock.clear();

                for _ in 0..loop_count {
                    if handle_command(
                        c,
                        &buffer,
                        &copy,
                        &secondary_buffer,
                        &mut is_string_mode_lock,
                        &buffers_cursor,
                        &buffers,
                    ) {
                        break;
                    }
                }
            }
        }
    }
}


fn handle_string_mode(
    c: char,
    buffer: &Arc<Mutex<buffer::ViewBuffer>>,
    is_escape_mode: &mut bool,
    is_string_mode: &mut bool,
    
) {
    let mut buffer_lock = buffer.lock().unwrap();
    match c {
        '\\' if !*is_escape_mode => *is_escape_mode = true,
        'n' if *is_escape_mode => {
            buffer_lock.add_char('\n');
            *is_escape_mode = false;
        }
        '\\' if *is_escape_mode => {
            buffer_lock.add_char('\\');
            *is_escape_mode = false;
        }
        ';' if *is_escape_mode => {
            buffer_lock.add_char(';');
            *is_escape_mode = false;
        }
        ';' if !*is_escape_mode => *is_string_mode = false,
        _ => buffer_lock.add_char(c),
    }
}

fn handle_command(
    c: char,
    buffer: &Arc<Mutex<buffer::ViewBuffer>>,
    copy: &Arc<Mutex<buffer::ViewBuffer>>,
    secondary_buffer: &Arc<Mutex<String>>,
    is_string_mode: &mut bool,
    buffers_cursor: &Arc<Mutex<usize>>,
    buffers: &Arc<Mutex<Vec<String>>>,
) -> bool {
    match c {
        'a' => *is_string_mode = true,
        'v' => {
            let buffer_lock = buffer.lock().unwrap();
            println!("{}", buffer_lock.buffer);
        }
        'V' => {
            let buffer_lock = buffer.lock().unwrap();
            print!("{}", buffer_lock.buffer);
        }
        'b' => buffer.lock().unwrap().cur_move_left(),
        'f' => buffer.lock().unwrap().cur_move_right(),
        'r' => buffer.lock().unwrap().remove_char(),
        'R' => {
            let mut buffer_lock = buffer.lock().unwrap();
            buffer_lock.buffer.clear();
            buffer_lock.cursor = 0;
        }
        'q' => process::exit(0),
        'l' => {
            let buffer_lock = buffer.lock().unwrap();
            *secondary_buffer.lock().unwrap() = buffer_lock.buffer.len().to_string();
        }
        'o' => {
            let mut buffer_lock = buffer.lock().unwrap();
            let mut secondary_buffer_lock = secondary_buffer.lock().unwrap();
            for c in secondary_buffer_lock.chars() {
                buffer_lock.add_char(c);
            }
            secondary_buffer_lock.clear();
        }
        'i' => {
            let buffer_lock = buffer.lock().unwrap();
            *secondary_buffer.lock().unwrap() = buffer_lock.buffer.clone();
        }
        's' => {
            let mut buffer_lock = buffer.lock().unwrap();
            let secondary_buffer_lock = secondary_buffer.lock().unwrap();
            if let Some(pos) = buffer_lock.buffer.find(&*secondary_buffer_lock) {
                buffer_lock.cursor = pos;
            }
        }
        '#' => return true,
        'F' => buffer.lock().unwrap().cursor = 0,
        'L' => {
            let mut buffer_lock = buffer.lock().unwrap();
            buffer_lock.cursor = buffer_lock.buffer.len();
        }
        'S' => {
            let buffer_lock = buffer.lock().unwrap();
            if let Ok(mut file) = fs::File::create(&buffer_lock.filename) {
                let _ = file.write_all(buffer_lock.buffer.as_bytes());
            }
        }
        '!' => {
            let secondary_buffer_lock = secondary_buffer.lock().unwrap();
            buffer.lock().unwrap().filename = secondary_buffer_lock.to_string();
        }
        '>' => {
            let buffer_lock = buffer.lock().unwrap();
            let mut copy_lock = copy.lock().unwrap();
            copy_lock.buffer = buffer_lock.buffer.clone();
            copy_lock.cur_move_right();
        }
        '<' => {
            let buffer_lock = buffer.lock().unwrap();
            let mut copy_lock = copy.lock().unwrap();
            copy_lock.buffer = buffer_lock.buffer.clone();
            copy_lock.cur_move_left();
        }
        'c' => {
            let buffer_lock = buffer.lock().unwrap();
            let mut copy_lock = copy.lock().unwrap();
            copy_lock.buffer = buffer_lock.buffer.clone();
            if let Some(cpbuf) = buffer_lock.buffer.get(buffer_lock.cursor..copy_lock.cursor) {
                copy_lock.buffer = cpbuf.into();
            }
        }
        'p' => {
            let mut buffer_lock = buffer.lock().unwrap();
            let copy_lock = copy.lock().unwrap();
            for c in copy_lock.buffer.chars() {
                buffer_lock.add_char(c);
            }
        }
        'P' => {
            let mut secondary_buffer_lock = secondary_buffer.lock().unwrap();
            let copy_lock = copy.lock().unwrap();
            for c in copy_lock.buffer.chars() {
                secondary_buffer_lock.push(c);
            }
        }
        'x' => {
            let mut buffer_lock = buffer.lock().unwrap();
            if fs::File::open(&buffer_lock.filename)
                .and_then(|mut file| file.read_to_string(&mut buffer_lock.buffer))
                .is_err()
            {
                println!("?");
            }
        }
        'z' => secondary_buffer.lock().unwrap().clear(),
        '&' => {
            let buffer_lock = buffer.lock().unwrap();
            let mut copy_lock = copy.lock().unwrap();
            copy_lock.buffer = buffer_lock.buffer.clone();
            copy_lock.cursor = buffer_lock.cursor;
        }
        '^' => {
            let mut buffer_lock = buffer.lock().unwrap();
            buffer_lock.cursor = copy.lock().unwrap().cursor;
        }
        '$' => {
            if fs::create_dir(&*secondary_buffer.lock().unwrap()).is_err() {
                println!("?");
            }
        }
        '%' => {
            if fs::remove_file(&*secondary_buffer.lock().unwrap()).is_err() {
                println!("?");
            }
        }
        '/' => {
            let buffers_cursor_ = buffers_cursor.lock().unwrap();
            let buffers_ = buffers.lock().unwrap();
            let str_new = &String::new();
            let macros = buffers_.get(*buffers_cursor_).unwrap_or(str_new);
            handle_char(&macros, buffer, &Arc::new(Mutex::new(false)), &Arc::new(Mutex::new(false)),
                copy, secondary_buffer, buffers_cursor, buffers, &Arc::new(Mutex::new(String::new())));
        }
        '+' => {
            let mut buffers_cursor = buffers_cursor.lock().unwrap();
            *buffers_cursor = ( *buffers_cursor + 1 ) % 10;
        }
        '-' => {
            let mut buffers_cursor = buffers_cursor.lock().unwrap();
            *buffers_cursor = ( *buffers_cursor - 1 ) % 10;
        }
        ':' => {
            let mut buffers = buffers.lock().unwrap();
            let buffers_cursor = buffers_cursor.lock().unwrap();
            buffers[*buffers_cursor] = (*secondary_buffer.lock().unwrap()).clone().to_string();
            
        }
        '~' => {
            let buffers = buffers.lock().unwrap();
            let buffers_cursor = buffers_cursor.lock().unwrap();
            let mut secondary_buffer = secondary_buffer.lock().unwrap();
            *secondary_buffer = (&*buffers[*buffers_cursor]).to_string();
        }
        '?' => {
            let buffers_cursor_ = buffers_cursor.lock().unwrap();
            let buffers_ = buffers.lock().unwrap();
            let str_new = &String::new();
            let macros = buffers_.get(*buffers_cursor_).unwrap_or(str_new);
            if (*buffer.lock().unwrap().buffer).contains(&*secondary_buffer.lock().unwrap()) {
                handle_char(&macros, buffer, &Arc::new(Mutex::new(false)), &Arc::new(Mutex::new(false)),
                copy, secondary_buffer, buffers_cursor, buffers, &Arc::new(Mutex::new(String::new())));
            }
        }

        '*' => {
            let buffers_cursor_ = buffers_cursor.lock().unwrap();
            let buffers_ = buffers.lock().unwrap();
            let str_new = &String::new();
            let macros = buffers_.get(*buffers_cursor_).unwrap_or(str_new);
            loop {
                handle_char(&macros, buffer, &Arc::new(Mutex::new(false)), &Arc::new(Mutex::new(false)),
                copy, secondary_buffer, buffers_cursor, buffers, &Arc::new(Mutex::new(String::new())));
            }
        }

        'w' => {
            thread::sleep(time::Duration::from_millis(10));
        }
        _ => (),
    }
    false
}
