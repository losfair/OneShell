use std;
use std::io::{Read, Write};
use std::ffi::CString;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::error::Error;
use cervus;
use serde_json;

#[derive(Default)]
pub struct Engine {
    last_exit_status: i32
}

#[derive(Deserialize)]
pub struct Block {
    ops: Vec<Operation>
}

#[derive(Deserialize)]
pub enum Operation {
    Exec(ExecInfo),
    ParallelExec(Vec<ExecInfo>),
    BackgroundExec(ExecInfo)
}

#[derive(Deserialize)]
pub struct ExecInfo {
    command: Vec<String>,
    env: HashMap<String, String>,
    stdin: StdioConfig,
    stdout: StdioConfig
}

#[derive(Deserialize, Eq, PartialEq)]
pub enum StdioConfig {
    Inherit,
    Pipe(String) // pipe name
}

#[derive(Debug)]
pub enum ExecError {
    Message(String)
}

impl Error for ExecError {
    fn description(&self) -> &str {
        match self {
            &ExecError::Message(ref m) => m.as_str()
        }
    }
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<&'static str> for ExecError {
    fn from(other: &'static str) -> ExecError {
        ExecError::Message(other.to_string())
    }
}

impl From<Box<Error>> for ExecError {
    fn from(other: Box<Error>) -> ExecError {
        ExecError::Message(other.description().to_string())
    }
}

impl Engine {
    pub fn new() -> Engine {
        Engine::default()
    }

    pub fn load_block(&mut self, ast: &str) -> Result<Block, Box<Error>> {
        Ok(serde_json::from_str(ast)?)
    }

    pub fn get_last_exit_status(&self) -> i32 {
        self.last_exit_status
    }

    pub fn eval_block(&mut self, blk: &Block) -> Result<(), Box<Error>> {
        for op in blk.ops.iter() {
            match op {
                &Operation::Exec(ref info) => {
                    self.handle_exec(info)?;
                },
                &Operation::ParallelExec(ref info) => {
                    self.handle_parallel_exec(info.as_slice())?;
                },
                &Operation::BackgroundExec(ref info) => {
                    self.handle_background_exec(info)?;
                }
            }
        }
        Ok(())
    }

    fn handle_exec(&mut self, info: &ExecInfo) -> Result<(), Box<Error>> {
        let mut cmd = build_command(info);

        let mut child = cmd.spawn()?;
        let exit_status = child.wait()?;

        self.last_exit_status = exit_status.code().unwrap_or(-1);

        Ok(())
    }

    fn handle_background_exec(&self, info: &ExecInfo) -> Result<(), Box<Error>> {
        let mut cmd = build_command(info);
        let mut child = cmd.spawn()?;

        std::thread::spawn(move || {
            match child.wait() {
                Ok(_) => {},
                Err(_) => {}
            }
        });

        Ok(())
    }

    fn handle_parallel_exec(&mut self, info: &[ExecInfo]) -> Result<(), Box<Error>> {
        let mut stdout_pipes: HashMap<String, Option<std::process::ChildStdout>> = HashMap::new();

        let mut children = Vec::new();
        for item in info.iter() {
            let mut cmd = build_command(item);
            let mut child = cmd.spawn()?;

            if let &StdioConfig::Pipe(ref name) = &item.stdout {
                stdout_pipes.insert(
                    name.clone(),
                    Some(std::mem::replace(&mut child.stdout, None).unwrap())
                );
            }

            children.push((child, item));
        }

        for &mut (ref mut child, info) in children.iter_mut() {
            if let &StdioConfig::Pipe(ref name) = &info.stdin {
                let target_stdin = std::mem::replace(&mut child.stdin, None).unwrap();
                let target_stdout = std::mem::replace(stdout_pipes.get_mut(name).unwrap(), None).unwrap();

                std::thread::spawn(move || {
                    let mut writer = std::io::BufWriter::new(target_stdin);
                    for b in target_stdout.bytes() {
                        if b.is_err() {
                            break;
                        }
                        match writer.write(&[b.unwrap()]) {
                            Ok(_) => {},
                            Err(_) => break
                        }
                    }
                });
            }
        }

        for mut child in children {
            let exit_status = child.0.wait()?;
            self.last_exit_status = exit_status.code().unwrap_or(-1);
        }

        Ok(())
    }
}

fn build_command(info: &ExecInfo) -> Command {
    let mut cmd = Command::new(info.command[0].as_str());
    for i in 1..info.command.len() {
        cmd.arg(info.command[i].as_str());
    }

    cmd.envs(&info.env);

    cmd.stdin(match info.stdin {
        StdioConfig::Inherit => Stdio::inherit(),
        StdioConfig::Pipe(_) => Stdio::piped()
    });

    cmd.stdout(match info.stdout {
        StdioConfig::Inherit => Stdio::inherit(),
        StdioConfig::Pipe(_) => Stdio::piped()
    });

    cmd
}
