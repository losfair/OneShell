use std;
use std::os::raw::c_char;
use std::ffi::CStr;
use engine;

#[no_mangle]
pub extern "C" fn oneshell_engine_create() -> *mut engine::EngineHandle {
    Box::into_raw(Box::new(engine::EngineHandle::from(
        engine::Engine::new()
    )))
}

#[no_mangle]
pub extern "C" fn oneshell_engine_destroy(eng: *mut engine::EngineHandle) {
    unsafe {
        Box::from_raw(eng);
    }
}

#[no_mangle]
pub extern "C" fn oneshell_engine_clone(eng: &engine::EngineHandle) -> *mut engine::EngineHandle {
    Box::into_raw(Box::new(engine::EngineHandle::from(
        eng.borrow().clone()
    )))
}

#[no_mangle]
pub extern "C" fn oneshell_block_load(ast: *const c_char) -> *mut engine::Block {
    let ast = unsafe {
        CStr::from_ptr(ast).to_str().unwrap()
    };
    match engine::Engine::load_block(ast) {
        Ok(v) => Box::into_raw(v),
        Err(e) => {
            eprintln!("{:?}", e);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn oneshell_block_destroy(blk: *mut engine::Block) {
    unsafe {
        Box::from_raw(blk);
    }
}

#[no_mangle]
pub extern "C" fn oneshell_engine_eval_block(eng: &mut engine::EngineHandle, blk: &mut engine::Block) -> i32 {
    eng.eval_block(blk)
}
