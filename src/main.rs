use std::env;
use std::fs::read_to_string;
use std::io::{self, Write};

use console::Term;

fn read_file(filename: &str) -> Vec<String> {
    let mut out_lines: Vec<String> = vec![];

    for line in read_to_string(filename).unwrap().lines() {
        out_lines.push(line.to_string())
    }

    return out_lines;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let filepath: Option<&String>;
    let mut strict: bool = false;

    if args.len() < 2 {
        eprintln!("Usage: {} [--strict] <filepath>", args[0]);
        std::process::exit(1);
    } else if args.len() == 2 {
        // filepath only
        filepath = Some(&args[1]);
    } else if args.len() == 3 {
        strict = args[1] == "--strict";
        filepath = Some(&args[2]);
    } else {
        eprintln!("Usage: {} [--strict] <filepath>", args[0]);
        std::process::exit(1);
    }

    run_brainfuck(
        tokenize_lines(read_file(&filepath.unwrap())),
        strict
    );
}

#[derive(Clone, PartialEq)]
struct Token {
    pub opcode: char,
    pub jump_addr: Option<usize>,
    pub line: usize,
}

impl Token {
    fn inst(opcode: char) -> Self {
        Self { opcode, jump_addr: None, line: 0 }
    }
}

fn tokenize_lines(lines: Vec<String>) -> Vec<Token> {
    let code_tokens: Vec<Token> = vec![
        Token::inst('<'),
        Token::inst('>'),
        Token::inst('+'),
        Token::inst('-'),
        Token::inst(','),
        Token::inst('.'),
        Token::inst('['),
        Token::inst(']'),
    ];
    let comment_tokens: Vec<char> = vec![
        '#', '/', ';'
    ];
    let mut opcode_tokens: Vec<Token> = vec![];
    let mut scope_open_addrs: Vec<usize> = vec![];

    for (line_num, line) in lines.iter().enumerate() {
        for character in line.chars() {
            let found_token = code_tokens.iter().find(
                |&c| c.opcode == character
            );

            if found_token == None {
                if comment_tokens.contains(&character) {
                    break; // comment start, skip to next line
                } else if !character.is_whitespace() {
                    println!("Unknown character on line {}, ignoring: {}", line_num + 1, character);
                }
            } else {
                match found_token.unwrap().opcode {
                    '[' => {
                        opcode_tokens.push(found_token.unwrap().clone());
                        scope_open_addrs.push(opcode_tokens.len() - 1);
                        opcode_tokens.last_mut().expect("Oops!").line = line_num + 1;
                    }
                    ']' => {
                        let scope_open_addr: usize = scope_open_addrs.pop().expect(
                            &format!("Tried to pop a scope that wasn't opened on line {}!", line_num + 1)
                        );
                        opcode_tokens.push(found_token.unwrap().clone());
                        opcode_tokens.last_mut().expect("Oops!").jump_addr = Some(scope_open_addr);
                        opcode_tokens.get_mut(scope_open_addr).expect("Oops!").jump_addr = Some(opcode_tokens.len() - 1);
                        opcode_tokens.last_mut().expect("Oops!").line = line_num + 1;
                    }
                    _ => {
                        opcode_tokens.push(found_token.unwrap().clone());
                        opcode_tokens.last_mut().expect("Oops!").line = line_num + 1;
                    }
                }
            }
        }
    }

    // ensure we have no dangling '['
    assert_eq!(scope_open_addrs.len(), 0);
    return opcode_tokens;
}

fn run_brainfuck(opcode_tokens: Vec<Token>, strict: bool) {
    let opcode_tokens: Vec<Token> = opcode_tokens.to_owned();
    let mut inst_ptr: usize = 0;
    let mut data_ptr: usize = 0;
    let mut data_cells: [u8; 32768] = [0; 32768];
    let data_size: usize = data_cells.len() - 1;
    let term: Term = Term::stdout();

    while inst_ptr < opcode_tokens.len() {
        let curr_inst: &Token = &opcode_tokens[inst_ptr];

        match curr_inst.opcode {
            '<' => { // decrement data pointer
                if data_ptr > 0 {
                    data_ptr -= 1;
                } else if strict {
                    panic!(
                        "\nAttempted data pointer underflow in strict mode at line {}.",
                        curr_inst.line
                    )
                } else {
                    data_ptr = data_size;
                }
                inst_ptr += 1;
            }
            '>' => { // increment data pointer
                if data_ptr < data_size {
                    data_ptr += 1;
                } else if strict {
                    panic!(
                        "\nAttempted data pointer overflow in strict mode at line {}.",
                        curr_inst.line
                    )
                } else {
                    data_ptr = 0;
                }
                inst_ptr += 1;
            }
            '+' => { // increment byte at data pointer
                if strict {
                    data_cells[data_ptr] = data_cells[data_ptr].checked_add(1).expect(
                        &format!(
                            "\nAttempted data cell overflow in strict mode at line {}.",
                            curr_inst.line
                        )
                    );
                } else {
                    data_cells[data_ptr] = data_cells[data_ptr].wrapping_add(1);
                }
                inst_ptr += 1;
            }
            '-' => { // decrement byte at data pointer
                if strict {
                    data_cells[data_ptr] = data_cells[data_ptr].checked_sub(1).expect(
                        &format!(
                            "\nAttempted data cell underflow in strict mode at line {}.",
                            curr_inst.line
                        )
                    );
                } else {
                    data_cells[data_ptr] = data_cells[data_ptr].wrapping_sub(1);
                }
                inst_ptr += 1;
            }
            '.' => { // output byte at data pointer
                print!("{}", data_cells[data_ptr] as char);
                io::stdout().flush().unwrap();
                inst_ptr += 1;
            }
            ',' => { // read one byte of input
                let in_buf: u8 = term.read_char().expect(
                    &format!("\nFailure to read char from terminal at line {}!", curr_inst.line)
                ) as u8;
                data_cells[data_ptr] = in_buf;
                inst_ptr += 1;
            }
            '[' => { // jump forward if data is zero
                if data_cells[data_ptr] == 0 {
                    inst_ptr = curr_inst.jump_addr.unwrap() + 1;
                } else {
                    inst_ptr += 1;
                }
            }
            ']' => { // jump back if data is non-zero
                if data_cells[data_ptr] != 0 {
                    inst_ptr = curr_inst.jump_addr.unwrap() + 1;
                } else {
                    inst_ptr += 1;
                }
            }
            _ => {
                println!("\nUnknown instruction at line {}, skipping: {}", curr_inst.line, curr_inst.opcode);
                inst_ptr += 1;
            }
        }
    }
}
