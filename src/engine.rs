use std;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{Read, Write};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::error::Error;
use std::ops::Deref;
use serde_json;
use backtrace;
use jit;
use signals;
use var;

// EngineHandle should never be moved as JIT-compiled code may hold reference to it.
pub struct EngineHandle {
    inner: Box<EngineHandleImpl>
}

pub struct EngineHandleImpl {
    inner: Rc<RefCell<Engine>>
}

impl Deref for EngineHandle {
    type Target = EngineHandleImpl;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl Deref for EngineHandleImpl {
    type Target = RefCell<Engine>;
    fn deref(&self) -> &RefCell<Engine> {
        &*self.inner
    }
}

impl From<Engine> for EngineHandle {
    fn from(other: Engine) -> EngineHandle {
        EngineHandle {
            inner: Box::new(EngineHandleImpl {
                inner: Rc::new(RefCell::new(other))
            })
        }
    }
}

#[derive(Default)]
pub struct Engine {
    pub last_exit_status: i32,
    pub call_stack: Vec<FunctionState>,
    pub vars: HashMap<String, var::Variable>
}

#[derive(Deserialize)]
pub struct Block {
    pub ops: Vec<Operation>,
    #[serde(skip)]
    pub jit_info: Option<jit::BlockJitInfo>,
    #[serde(skip)]
    call_count_before_jit: usize
}

impl Clone for Block {
    fn clone(&self) -> Block {
        Block {
            ops: self.ops.clone(),
            jit_info: None,
            call_count_before_jit: 0
        }
    }
}

pub struct FunctionState {
    pub vars: HashMap<String, var::Variable>
}

#[derive(Deserialize, Clone)]
pub enum Operation {
    Exec(ExecInfo),
    ParallelExec(Vec<ExecInfo>),
    BackgroundExec(ExecInfo),
    IfElse(Block, Block),
    Loop(Block),
    Break,
    AssignGlobal(String, var::Value),
    AssignLocal(String, var::Value),
    EngineBacktrace,
    Print(StringSource)
}

#[derive(Deserialize, Clone)]
pub struct ExecInfo {
    command: Vec<StringSource>,
    env: HashMap<String, String>,
    stdin: StdioConfig,
    stdout: StdioConfig
}

#[derive(Deserialize, Clone)]
pub enum StringSource {
    Plain(String),
    GlobalVariable(String),
    LocalVariable(String)
}

impl StringSource {
    pub fn fetch(&self, eng: &Engine) -> Option<String> {
        match *self {
            StringSource::Plain(ref v) => Some(v.clone()),
            StringSource::GlobalVariable(ref name) => match eng.vars.get(name) {
                Some(v) => Some(v.to_string()),
                None => None
            },
            StringSource::LocalVariable(ref name) => match eng.call_stack.last().unwrap().vars.get(name) {
                Some(v) => Some(v.to_string()),
                None => None
            }
        }
    }
}

impl ExecInfo {
    pub fn build(&self, eng: &Engine) -> Result<Command, Box<Error>> {
        let mut cmd = Command::new(match self.command[0].fetch(eng) {
            Some(v) => v,
            None => return Err("Invalid first argument".into())
        });
        for i in 1..self.command.len() {
            cmd.arg(match self.command[i].fetch(eng) {
                Some(v) => v,
                None => return Err("Invalid argument found in parameter list".into())
            });
        }

        cmd.envs(&self.env);

        cmd.stdin(match self.stdin {
            StdioConfig::Inherit => Stdio::inherit(),
            StdioConfig::Pipe(_) => Stdio::piped()
        });

        cmd.stdout(match self.stdout {
            StdioConfig::Inherit => Stdio::inherit(),
            StdioConfig::Pipe(_) => Stdio::piped()
        });

        Ok(cmd)
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
    pub fn impl_ref(&self) -> &EngineHandleImpl {
        &*self.inner
    }
}

impl EngineHandleImpl {
    pub fn engine_rc(&self) -> Rc<RefCell<Engine>> {
        self.inner.clone()
    }

    pub fn eval_block(&self, blk: &mut Block) -> i32 {
        if blk.jit_info.is_some() {
            //println!("JIT HIT");
            let entry = blk.jit_info.as_ref().unwrap().entry;
            entry()
        } else {
            //println!("JIT MISS");
            let mut ret: i32 = signals::OK;

            for op in blk.ops.iter_mut() {
                ret = self.eval_op(op);
                if ret != signals::OK {
                    break;
                }
            }
            blk.call_count_before_jit += 1;
            if blk.call_count_before_jit == 3 {
                blk.build_jit(self).unwrap();
            }
            ret
        }
    }

    pub fn eval_op(&self, op: &mut Operation) -> i32 {
        match op {
            &mut Operation::Exec(ref info) => {
                self.borrow_mut().handle_exec(info).unwrap();
                signals::OK
            },
            &mut Operation::ParallelExec(ref info) => {
                self.borrow_mut().handle_parallel_exec(info.as_slice()).unwrap();
                signals::OK
            },
            &mut Operation::BackgroundExec(ref info) => {
                self.borrow_mut().handle_background_exec(info).unwrap();
                signals::OK
            },
            &mut Operation::IfElse(ref mut if_blk, ref mut else_blk) => {
                if self.borrow().last_exit_status == 0 {
                    self.eval_block(else_blk)
                } else {
                    self.eval_block(if_blk)
                }
            },
            &mut Operation::Loop(ref mut blk) => {
                loop {
                    let ret = self.eval_block(blk);
                    if ret == signals::OK {
                        continue;
                    } else if ret == signals::BREAK {
                        break;
                    } else if ret == signals::CONTINUE {
                        continue;
                    } else {
                        return ret;
                    }
                }
                signals::OK
            },
            &mut Operation::Break => {
                signals::BREAK
            },
            &mut Operation::AssignGlobal(ref name, ref val) => {
                self.borrow_mut().vars.insert(
                    name.clone(),
                    var::Variable::from_value(val.clone())
                );
                signals::OK
            },
            &mut Operation::AssignLocal(ref name, ref val) => {
                self.borrow_mut().call_stack.last_mut().unwrap().vars.insert(
                    name.clone(),
                    var::Variable::from_value(val.clone())
                );
                signals::OK
            },
            &mut Operation::EngineBacktrace => {
                self.borrow().handle_engine_backtrace();
                signals::OK
            },
            &mut Operation::Print(ref src) => {
                self.borrow().handle_print(src);
                signals::OK
            }
        }
    }
}

impl Engine {
    pub fn new() -> Engine {
        Engine::default()
    }

    // Block must be boxed to prevent move
    pub fn load_block(ast: &str) -> Result<Box<Block>, Box<Error>> {
        Ok(Box::new(serde_json::from_str(ast)?))
    }

    pub fn get_last_exit_status(&self) -> i32 {
        self.last_exit_status
    }

    pub fn handle_exec(&mut self, info: &ExecInfo) -> Result<(), Box<Error>> {
        let mut cmd = info.build(self)?;

        let mut child = cmd.spawn()?;
        let exit_status = child.wait()?;

        self.last_exit_status = exit_status.code().unwrap_or(-1);

        Ok(())
    }

    pub fn handle_background_exec(&self, info: &ExecInfo) -> Result<(), Box<Error>> {
        let mut cmd = info.build(self)?;
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
            let mut cmd: Command = item.build(self)?;
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

    pub fn handle_print(&self, src: &StringSource) {
        println!("{}", match src.fetch(self) {
            Some(v) => v,
            None => "(undefined)".to_string()
        });
    }

    pub fn handle_engine_backtrace(&self) {
        let bt = backtrace::Backtrace::new();
        println!("{:?}", bt);
    }
}
