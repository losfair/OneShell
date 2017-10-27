use std;
use std::any::Any;
use std::error::Error;
use std::process::Command;
use std::os::raw::c_void;
use engine;
use cervus;
use cervus::engine::Action;
use cervus::value_type::ValueType;
use engine::Operation;

pub struct BlockJitInfo {
    resources: Vec<Box<Any>>,
    ee: cervus::engine::ExecutionEngine,
    pub entry: extern fn ()
}

impl engine::Block {
    pub fn build_jit(&mut self, eh: &engine::EngineHandle) -> Result<(), Box<Error>> {
        let mut resources: Vec<Box<Any>> = Vec::new();
        let m = cervus::engine::Module::new("");

        let entry;

        {
            let entry_fn = cervus::engine::Function::new(
                &m,
                "entry",
                ValueType::Void,
                vec![]
            );
            let mut bb = cervus::engine::BasicBlock::new(&entry_fn, "");
            let mut new_bb: Option<cervus::engine::BasicBlock> = None;

            

            let handle_exec_wrapper_fn = cervus::engine::Value::from(handle_exec_wrapper as *const c_void as u64)
                .const_int_to_ptr(ValueType::Pointer(Box::new(
                    ValueType::Function(
                        Box::new(ValueType::Void),
                        vec![
                            ValueType::Pointer(Box::new(ValueType::Void)),
                            ValueType::Pointer(Box::new(ValueType::Void))
                        ]
                    )
                )));

            let handle_parallel_exec_wrapper_fn = cervus::engine::Value::from(handle_parallel_exec_wrapper as *const c_void as u64)
                .const_int_to_ptr(ValueType::Pointer(Box::new(
                    ValueType::Function(
                        Box::new(ValueType::Void),
                        vec![
                            ValueType::Pointer(Box::new(ValueType::Void)),
                            ValueType::Pointer(Box::new(ValueType::Void))
                        ]
                    )
                )));

            let handle_background_exec_wrapper_fn = cervus::engine::Value::from(handle_background_exec_wrapper as *const c_void as u64)
                .const_int_to_ptr(ValueType::Pointer(Box::new(
                    ValueType::Function(
                        Box::new(ValueType::Void),
                        vec![
                            ValueType::Pointer(Box::new(ValueType::Void)),
                            ValueType::Pointer(Box::new(ValueType::Void))
                        ]
                    )
                )));

            let call_block_wrapper_fn = cervus::engine::Value::from(call_block_wrapper as *const c_void as u64)
                .const_int_to_ptr(ValueType::Pointer(Box::new(
                    ValueType::Function(
                        Box::new(ValueType::Void),
                        vec![
                            ValueType::Pointer(Box::new(ValueType::Void)),
                            ValueType::Pointer(Box::new(ValueType::Void))
                        ]
                    )
                )));

            resources.push(Box::new(eh.clone()) as Box<Any>);

            for op in self.ops.iter_mut() {
                if new_bb.is_some() {
                    bb = std::mem::replace(&mut new_bb, None).unwrap();
                }
                let builder = cervus::engine::Builder::new(&bb);

                match op {
                    &mut Operation::Exec(ref info) => {
                        let info = Box::new(info.clone());

                        builder.append(
                            Action::Call(
                                handle_exec_wrapper_fn.clone(),
                                vec![
                                    cervus::engine::Value::from(&*eh.borrow() as *const engine::Engine as u64).const_int_to_ptr(
                                        ValueType::Pointer(Box::new(ValueType::Void))
                                    ),
                                    cervus::engine::Value::from(&*info as *const engine::ExecInfo as u64).const_int_to_ptr(
                                        ValueType::Pointer(Box::new(ValueType::Void))
                                    )
                                ]
                            )
                        );
                        resources.push(info as Box<Any>);
                    },
                    &mut Operation::ParallelExec(ref info) => {
                        let info = Box::new(info.to_vec());

                        builder.append(
                            Action::Call(
                                handle_parallel_exec_wrapper_fn.clone(),
                                vec![
                                    cervus::engine::Value::from(&*eh.borrow() as *const engine::Engine as u64).const_int_to_ptr(
                                        ValueType::Pointer(Box::new(ValueType::Void))
                                    ),
                                    cervus::engine::Value::from(&*info as *const Vec<engine::ExecInfo> as u64).const_int_to_ptr(
                                        ValueType::Pointer(Box::new(ValueType::Void))
                                    )
                                ]
                            )
                        );
                        resources.push(info as Box<Any>);
                    },
                    &mut Operation::BackgroundExec(ref info) => {
                        let info = Box::new(info.clone());

                        builder.append(
                            Action::Call(
                                handle_background_exec_wrapper_fn.clone(),
                                vec![
                                    cervus::engine::Value::from(&*eh.borrow() as *const engine::Engine as u64).const_int_to_ptr(
                                        ValueType::Pointer(Box::new(ValueType::Void))
                                    ),
                                    cervus::engine::Value::from(&*info as *const engine::ExecInfo as u64).const_int_to_ptr(
                                        ValueType::Pointer(Box::new(ValueType::Void))
                                    )
                                ]
                            )
                        );
                        resources.push(info as Box<Any>);
                    },
                    &mut Operation::IfElse(ref mut if_blk, ref mut else_blk) => {
                        let last_exit_status_ptr = &eh.borrow().last_exit_status as *const i32;
                        let last_exit_status_ptr_handle = cervus::engine::Value::from(last_exit_status_ptr as u64)
                            .const_int_to_ptr(ValueType::Pointer(Box::new(ValueType::Int32)));

                        let if_bb = cervus::engine::BasicBlock::new(&entry_fn, "");
                        let else_bb = cervus::engine::BasicBlock::new(&entry_fn, "");
                        let cont_bb = cervus::engine::BasicBlock::new(&entry_fn, "");

                        builder.append(
                            Action::ConditionalBranch(
                                builder.append(
                                    Action::IntNotEqual(
                                        builder.append(
                                            Action::Load(last_exit_status_ptr_handle)
                                        ),
                                        (0 as i32).into()
                                    )
                                ),
                                &if_bb,
                                &else_bb
                            )
                        );

                        {
                            let if_builder = cervus::engine::Builder::new(&if_bb);

                            if_builder.append(
                                Action::Call(
                                    call_block_wrapper_fn.clone(),
                                    vec![
                                        cervus::engine::Value::from(eh as *const engine::EngineHandle as u64).const_int_to_ptr(
                                            ValueType::Pointer(Box::new(ValueType::Void))
                                        ),
                                        cervus::engine::Value::from(if_blk as *const engine::Block as u64).const_int_to_ptr(
                                            ValueType::Pointer(Box::new(ValueType::Void))
                                        ),
                                    ]
                                )
                            );
                            if_builder.append(Action::Branch(&cont_bb));
                        }

                        {
                            let else_builder = cervus::engine::Builder::new(&else_bb);
                            else_builder.append(
                                Action::Call(
                                    call_block_wrapper_fn.clone(),
                                    vec![
                                        cervus::engine::Value::from(eh as *const engine::EngineHandle as u64).const_int_to_ptr(
                                            ValueType::Pointer(Box::new(ValueType::Void))
                                        ),
                                        cervus::engine::Value::from(else_blk as *const engine::Block as u64).const_int_to_ptr(
                                            ValueType::Pointer(Box::new(ValueType::Void))
                                        ),
                                    ]
                                )
                            );
                            else_builder.append(Action::Branch(&cont_bb));
                        }

                        new_bb = Some(cont_bb);
                    }
                }
            }

            if new_bb.is_some() {
                bb = std::mem::replace(&mut new_bb, None).unwrap();
            }
            let builder = cervus::engine::Builder::new(&bb);

            builder.append(Action::ReturnVoid);
            entry = entry_fn.to_null_handle();
        }

        let ee = cervus::engine::ExecutionEngine::new(m);
        ee.prepare();

        let entry = ee.get_callable_0::<()>(&entry);

        let mut jit_info = BlockJitInfo {
            resources: resources,
            ee: ee,
            entry: entry
        };

        self.jit_info = Some(jit_info);
        Ok(())
    }
}

extern "C" fn handle_exec_wrapper(eng: &mut engine::Engine, info: &engine::ExecInfo) {
    eng.handle_exec(info).unwrap();
}

extern "C" fn handle_background_exec_wrapper(eng: &mut engine::Engine, info: &engine::ExecInfo) {
    eng.handle_background_exec(info).unwrap();
}

extern "C" fn handle_parallel_exec_wrapper(eng: &mut engine::Engine, info: &Vec<engine::ExecInfo>) {
    eng.handle_parallel_exec(info.as_slice()).unwrap();
}

extern "C" fn call_block_wrapper(eng: &engine::EngineHandle, blk: &mut engine::Block) {
    eng.eval_block(blk).unwrap();
}
