use std;
use std::os::raw::c_void;
use std::error::Error;
use std::rc::Rc;
use std::cell::{RefCell, Ref, RefMut};
use engine;
use signals;

#[derive(Deserialize, Clone, PartialEq)]
pub enum Value {
    Null,
    Integer(i64),
    Float(f64),
    String(String),
    Function(engine::Block)
}

#[derive(Clone)]
pub struct Variable {
    inner: Rc<RefCell<VariableImpl>>
}

pub struct VariableImpl {
    pub value: Value
}

impl Value {
    pub fn to_string(&self) -> String {
        match *self {
            Value::Null => "(null)".to_string(),
            Value::Integer(v) => format!("{}", v),
            Value::Float(v) => format!("{}", v),
            Value::String(ref v) => v.clone(),
            Value::Function(_) => "<Function>".to_string()
        }
    }
}

impl Variable {
    pub fn from_value(val: Value) -> Variable {
        Variable {
            inner: Rc::new(RefCell::new(VariableImpl {
                value: val
            }))
        }
    }

    pub fn impl_ref(&self) -> Ref<VariableImpl> {
        self.inner.borrow()
    }

    pub fn impl_ref_mut(&self) -> RefMut<VariableImpl> {
        self.inner.borrow_mut()
    }

    pub fn to_string(&self) -> String {
        self.inner.borrow().value.to_string()
    }

    pub fn deep_clone(&self) -> Variable {
        Variable::from_value(self.inner.borrow().value.clone())
    }

    // FIXME: Extremely unsafe code
    pub fn call(&self, eng: &engine::EngineHandle) -> Result<(), Box<Error>> {
        if let Value::Function(ref blk) = self.inner.borrow().value {
            eng.borrow_mut().call_stack.push(Box::new(engine::FunctionState::new()));

            let _eng = eng as *const engine::EngineHandle as *const c_void;
            let blk = blk as *const engine::Block as *const c_void;

            let ret = match std::panic::catch_unwind(|| {
                let eng = unsafe { &*(_eng as *const engine::EngineHandle) };
                let blk = unsafe { &mut *(blk as *mut engine::Block) };

                eng.eval_block(blk)
            }) {
                Ok(ret) => if ret == signals::OK {
                    Ok(())
                } else {
                    Err("Bad control status".into())
                },
                Err(_) => Err("Error in function".into())
            };

            eng.borrow_mut().call_stack.pop();

            ret
        } else {
            Err("Value cannot be called as a function".into())
        }
    }
}
