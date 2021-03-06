use std::process::{Stdio, Command, Child};
use std::os::windows::io::{FromRawHandle, AsRawHandle};
use std::io;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

///Split Pipes
///Given a command with pipes in it, it will split them into separate
///commands to be executed
fn split_pipes(input: Vec<String>) -> Vec<Vec<String>> {
    let input_slice = input.as_slice();
    let mut pipe_commands: Vec<Vec<String>> = Vec::new();
    let mut temp: Vec<String> = Vec::new();
    for i in input_slice {
        if i.contains('|') {
            let mut splits = i.split('|');
            temp.push(splits.next()
                .expect("Failed to split pipes").to_string());
            if temp.last()
                .expect("Called last on an empty vec") == &"" {
                temp.pop();
            }
            pipe_commands.push(temp.clone());
            temp.clear();
            temp.push(splits.next()
                .expect("Unwrapped a non existent value").to_string());
            if temp.last()
                .expect("Unwrapped an empty list") == &"" {
                temp.pop();
            }
        } else {
            temp.push(i.to_string());
        }
    }
    pipe_commands.push(temp);
    pipe_commands
}

///Piped
///The logic of piping is done here and calls the functions that execute
///the pipes and returns the result
pub fn piped(input: Vec<String>) -> bool {
    let mut split = split_pipes(input);
    let mut child_result = first_pipe(split.remove(0));
    let mut child: Child;

    child = child_result.expect("Failed to unwrap an Result");

    while split.len() > 1 {
        child_result = execute_pipe(split.remove(0), child);
        child = child_result.expect("Failed to unwrap an Result");
    }

    final_pipe(split.remove(0), child)
}

pub fn piped_redirect_out(input: Vec<String>) -> bool {
    let mut split = split_pipes(input);
    let mut child_result = first_pipe(split.remove(0));
    let mut child: Child;

    child = child_result.expect("Failed to unwrap an Result");

    while split.len() > 1 {
        child_result = execute_pipe(split.remove(0), child);
        child = child_result.expect("Failed to unwrap an Result");
    }

    final_piped_redirect(split.remove(0), child)
}

pub fn piped_detached(_: Vec<String>) -> bool {
    println!("Not implemented on this platform yet");
    false
}

pub fn piped_redirect_out_detached(_: Vec<String>) -> bool {
    println!("Not implemented on this platform yet");
    false
}

///First Pipe
///Always executed if piping and returns the child process to be used
///for the next pipe.
fn first_pipe(command: Vec<String>) -> io::Result<Child> {
    let args = command.as_slice();
    if args.len() > 1 {
        Command::new(&args[0]).args(&args[1..])
            .stdout(Stdio::piped()).spawn()
    } else if args.len() == 1 {
        Command::new(&args[0])
            .stdout(Stdio::piped()).spawn()
    } else {
        Command::new("")
            .stdout(Stdio::piped()).spawn()
    }
}

///Execute Pipe
///Used if there are more than two commands with piping. Takes a Child process
///as input for the next pipe and returns a Child process.
fn execute_pipe(command: Vec<String>, child: Child) -> io::Result<Child> {
    let args = command.as_slice();
    unsafe {
        if args.len() > 1 {
            Command::new(&args[0]).args(&args[1..])
                .stdout(Stdio::piped())
                .stdin(Stdio::from_raw_handle(child.stdout
                    .expect("No stdout").as_raw_handle()))
                .spawn()
        } else if args.len() == 1 {
            Command::new(&args[0])
                .stdout(Stdio::piped())
                .stdin(Stdio::from_raw_handle(child.stdout
                    .expect("No stdout").as_raw_handle()))
                .spawn()
        } else {
            Command::new("")
                .stdout(Stdio::piped())
                .stdin(Stdio::from_raw_handle(child.stdout
                    .expect("No stdout").as_raw_handle()))
                .spawn()
        }
    }
}

///Final Pipe
///Always executed when piping processes. Takes a child process as input
///and returns the output of piping the commands.
fn final_pipe(command: Vec<String>, child: Child) -> bool {
    let args = command.as_slice();
    unsafe {
        if args.len() > 1 {
            let mut child = Command::new(&args[0])
                .args(&args[1..])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .stdin(Stdio::from_raw_handle(child.stdout
                    .expect("No stdout for child process")
                    .as_raw_handle()))
                .spawn()
                .expect("Command failed to start");
            let status = child.wait().expect("failed to wait for child");;
            status.success()
        } else if args.len() == 1 {
            let mut child = Command::new(&args[0])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .stdin(Stdio::from_raw_handle(child.stdout
                    .expect("No stdout for child process")
                    .as_raw_handle()))
                .spawn()
                .expect("Command failed to start");
            let status = child.wait().expect("failed to wait for child");
            status.success()
        } else {
            let mut child = Command::new("")
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .stdin(Stdio::from_raw_handle(child.stdout
                    .expect("No stdout for child process")
                    .as_raw_handle()))
                .spawn()
                .expect("Command failed to start");
            let status = child.wait().expect("failed to wait for child");
            status.success()
        }
    }
}

fn final_piped_redirect(command: Vec<String>, child: Child) -> bool {
    let mut args = command;
    let mut file_path = "".to_owned();
    for i in 0..args.len() {
        if args[i].contains('>') {
            file_path.push_str(&args[i + 1..args.len()].to_vec().join(""));
            args.truncate(i);
            break;
        }
    }
    let args = args.as_slice();
    unsafe {
        let output = if args.len() > 1 {
            Command::new(&args[0])
                .args(&args[1.. ])
                .stdout(Stdio::piped())
                .stdin(Stdio::from_raw_handle(child.stdout
                    .expect("No stdout for child process")
                    .as_raw_handle()))
                .output()
                .ok()
        } else if args.len() == 1 {
            Command::new(&args[0])
                .stdout(Stdio::piped())
                .stdin(Stdio::from_raw_handle(child.stdout
                    .expect("No stdout for child process")
                    .as_raw_handle()))
                .output()
                .ok()
        } else {
            Command::new("")
                .stdout(Stdio::piped())
                .stdin(Stdio::from_raw_handle(child.stdout
                    .expect("No stdout for child process")
                    .as_raw_handle()))
                .output()
                .ok()
        };
        let str_out = if output.is_some() {
            let temp = output.expect("Output has been checked");
            if temp.stdout.is_empty() {
                String::from_utf8(temp.stderr)
                    .expect("Should have translated to string easily")
            } else {
                String::from_utf8(temp.stdout)
                    .expect("Should have translated to string easily")
            }
        } else {
            "".to_owned()
        };
        let path = Path::new(&file_path);
        let display = path.display();
        let mut file = match File::create(&path) {
            Err(why) => panic!("couldn't open {}: {}", display, why.description()),
            Ok(file) => file,
        };
        if let Err(why) = file.write_all(str_out.as_bytes()) {
            panic!("couldn't write to {}: {}", display, why.description());
        }
    }
    true
}

