use std::rc::Rc;
use std::cell::{RefCell, RefMut};
use std::ops::Deref;

#[derive(Deserialize, Clone)]
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

impl Variable {
    pub fn from_value(val: Value) -> Variable {
        Variable {
            inner: Rc::new(RefCell::new(VariableImpl {
                value: val
            }))
        }
    }

    pub fn impl_ref_mut(&self) -> RefMut<VariableImpl> {
        self.inner.borrow_mut()
    }
}
