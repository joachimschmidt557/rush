#![allow(unused_must_use)]

extern crate rush;
extern crate rustyline;
extern crate libc;
extern crate nix;
extern crate dirs;
extern crate clap;

use rush::builtins;
use rush::prompt::Prompt;
use rush::interpreter::*;
use rush::script::*;
use rustyline::error::ReadlineError;
use rustyline::{Config, CompletionType, Editor, Helper};
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use self::dirs::home_dir;
use std::process;
use std::path::Path;
use nix::sys::signal;
use nix::sys::signal::{SigAction, SigHandler, SaFlags, SigSet, sigaction};
use std::borrow::Cow::{self, Borrowed, Owned};
use clap::{Arg, App};


struct RushHelper(FilenameCompleter);

impl Completer for RushHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize) -> Result<(usize, Vec<Pair>), ReadlineError> {
        self.0.complete(line, pos)
    }
}

impl Highlighter for RushHelper {
    fn highlight_prompt<'p>(&self, prompt: &'p str) -> Cow<'p, str> {
        Borrowed(prompt)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }
}

impl Hinter for RushHelper {
    fn hint(&self, line: &str, _pos: usize) -> Option<String> {
        if line == "hello" {
            Some(" World".to_owned())
        } else {
            None
        }
    }
}

impl Helper for RushHelper {}

fn main() {
    #[cfg(unix)]    {
        while nix::unistd::tcgetpgrp(0).unwrap() != nix::unistd::getpgrp() {
            nix::sys::signal::kill(nix::unistd::getpgrp(), nix::sys::signal::Signal::SIGTTIN);
        }
        let hdl = SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty());
        unsafe {
            sigaction(signal::SIGINT, &hdl).unwrap();
            sigaction(signal::SIGQUIT, &hdl).unwrap();
            sigaction(signal::SIGTSTP, &hdl).unwrap();
            sigaction(signal::SIGTTIN, &hdl).unwrap();
            sigaction(signal::SIGTTOU, &hdl).unwrap();
            sigaction(signal::SIGTSTP, &hdl).unwrap();
        }
        let pid = nix::unistd::getpid();
        match nix::unistd::setpgid(pid, pid) {
            Ok(_) => {}
            Err(_) => println!("Couldn't set pgid"),
        };
        match nix::unistd::tcsetpgrp(0, pid) {
            Ok(_) => {}
            Err(_) => println!("Couldn't set process to foreground"),
        }
    }

    // Parse command line options
    let matches = App::new("rush")
        .version("0.0.2")
        .about("Rust Shell")
        .arg(Arg::with_name("command")
            .short("c")
            .value_name("command")
            .multiple(true)
            .help("Command(s) to parse"))
        .arg(Arg::with_name("file")
            .short("f")
            .value_name("file")
            .multiple(true)
            .help("Files to run"))
        .arg(Arg::with_name("command_file")
            .multiple(true)
            .help("Commands or files"))
        .get_matches();

    // Load builtins
    let builtins = builtins::get_builtins();

    // Run config file
    let mut home_config = home_dir().expect("No Home directory");
    home_config.push(".rushrc");
    run_script(home_config.as_path(), &builtins);

    // Run script(s)
    for command_or_file in matches.value_of("command_file") {
        run_script(Path::new(&command_or_file), &builtins);
        return
    }

    // Run command(s)
    for command in matches.value_of("command") {
        interpret_line(command.to_string(), &builtins);
        return
    }

    let mut history_file = home_dir().expect("No Home directory");
    history_file.push(".rush_history");
    let history =
        history_file.as_path().to_str().expect("Should have a home directory to turn into a str");

    // Set up buffer to read inputs and History Buffer
    let input_config = Config::builder().completion_type(CompletionType::List).build();
    let mut input_buffer = Editor::with_config(input_config);
    input_buffer.set_helper(Some(RushHelper(FilenameCompleter::new())));
    if let Err(_) = input_buffer.load_history(history) {}
    let mut prompt = Prompt::new();

    // Loop to recieve and execute commands
    loop {
        &prompt.update_cwd();
        &prompt.update_prompt();
        let line = input_buffer.readline(&prompt.get_user_p());
        match line {
            Ok(line) => {
                input_buffer.add_history_entry(line.as_ref());
                interpret_line(line, &builtins);
            }
            Err(ReadlineError::Interrupted) => {
                print!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("exit");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                process::exit(1);
            }
        }
    }
    input_buffer.save_history(history).unwrap();
}
