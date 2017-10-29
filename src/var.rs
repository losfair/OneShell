use std::rc::Rc;
use std::cell::{RefCell, Ref, RefMut};

#[derive(Deserialize, Clone, PartialEq)]
pub enum Value {
    Null,
    Integer(i64),
    Float(f64),
    String(String)
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
            Value::String(ref v) => v.clone()
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
}
