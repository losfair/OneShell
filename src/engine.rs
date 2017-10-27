use std;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{Read, Write};
use std::ffi::CString;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::error::Error;
use std::ops::Deref;
use cervus;
use serde_json;
use jit;

// EngineHandle should never be moved as JIT-compiled code may hold reference to it.
#[derive(Clone)]
pub struct EngineHandle {
    inner: Rc<RefCell<Engine>>
}

impl Deref for EngineHandle {
    type Target = RefCell<Engine>;
    fn deref(&self) -> &RefCell<Engine> {
        &*self.inner
    }
}

impl From<Engine> for EngineHandle {
    fn from(other: Engine) -> EngineHandle {
        EngineHandle {
            inner: Rc::new(RefCell::new(other))
        }
    }
}

#[derive(Default)]
pub struct Engine {
    pub last_exit_status: i32
}

#[derive(Deserialize)]
pub struct Block {
    pub ops: Vec<Operation>,
    #[serde(skip)]
    pub jit_info: Option<jit::BlockJitInfo>,
    #[serde(skip)]
    call_count_before_jit: usize
}

#[derive(Deserialize)]
pub enum Operation {
    Exec(ExecInfo),
    ParallelExec(Vec<ExecInfo>),
    BackgroundExec(ExecInfo),
    IfElse(Block, Block)
}

#[derive(Deserialize, Clone)]
pub struct ExecInfo {
    command: Vec<String>,
    env: HashMap<String, String>,
    stdin: StdioConfig,
    stdout: StdioConfig
}

impl<'a> Into<Command> for &'a ExecInfo {
    fn into(self) -> Command {
        build_command(self)
    }
}

impl Into<Command> for ExecInfo {
    fn into(self) -> Command {
        build_command(&self)
    }
}

#[derive(Deserialize, Eq, PartialEq, Clone)]
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

impl EngineHandle {
    pub fn eval_block(&self, blk: &mut Block) -> Result<(), Box<Error>> {
        if blk.jit_info.is_some() {
            //println!("JIT HIT");
            let entry = blk.jit_info.as_ref().unwrap().entry;
            match std::panic::catch_unwind(|| {
                entry();
            }) {
                Ok(_) => {},
                Err(_) => return Err("Error in JIT-compiled code".into())
            }
        } else {
            //println!("JIT MISS");
            for op in blk.ops.iter_mut() {
                self.eval_op(op)?;
            }
            blk.call_count_before_jit += 1;
            if blk.call_count_before_jit == 3 {
                blk.build_jit(self);
            }
        }
        Ok(())
    }

    pub fn eval_op(&self, op: &mut Operation) -> Result<(), Box<Error>> {
        match op {
            &mut Operation::Exec(ref info) => {
                self.borrow_mut().handle_exec(info)?;
            },
            &mut Operation::ParallelExec(ref info) => {
                self.borrow_mut().handle_parallel_exec(info.as_slice())?;
            },
            &mut Operation::BackgroundExec(ref info) => {
                self.borrow_mut().handle_background_exec(info)?;
            },
            &mut Operation::IfElse(ref mut if_blk, ref mut else_blk) => {
                if self.borrow().last_exit_status == 0 {
                    self.eval_block(else_blk)?;
                } else {
                    self.eval_block(if_blk)?;
                }
            }
        }

        Ok(())
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

    pub fn handle_exec<T: Into<Command>>(&mut self, info: T) -> Result<(), Box<Error>> {
        let mut cmd = info.into();

        let mut child = cmd.spawn()?;
        let exit_status = child.wait()?;

        self.last_exit_status = exit_status.code().unwrap_or(-1);

        Ok(())
    }

    pub fn handle_background_exec<T: Into<Command>>(&self, info: T) -> Result<(), Box<Error>> {
        let mut cmd = info.into();
        let mut child = cmd.spawn()?;

        std::thread::spawn(move || {
            match child.wait() {
                Ok(_) => {},
                Err(_) => {}
            }
        });

        Ok(())
    }

    pub fn handle_parallel_exec(&mut self, info: &[ExecInfo]) -> Result<(), Box<Error>> {
        let mut stdout_pipes: HashMap<String, Option<std::process::ChildStdout>> = HashMap::new();

        let mut children = Vec::new();
        for item in info.iter() {
            let mut cmd: Command = item.into();
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

pub fn build_command(info: &ExecInfo) -> Command {
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