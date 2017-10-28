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
use signals;

pub struct BlockJitInfo {
    resources: Vec<Box<Any>>,
    ee: cervus::engine::ExecutionEngine,
    pub entry: extern fn () -> i32
}

impl engine::Block {
    pub fn build_jit(&mut self, eh: &engine::EngineHandleImpl) -> Result<(), Box<Error>> {
        let mut resources: Vec<Box<Any>> = Vec::new();
        let m = cervus::engine::Module::new("");

        let entry;

        {
            let entry_fn = cervus::engine::Function::new(
                &m,
                "entry",
                ValueType::Int32,
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
                        Box::new(ValueType::Int32),
                        vec![
                            ValueType::Pointer(Box::new(ValueType::Void)),
                            ValueType::Pointer(Box::new(ValueType::Void))
                        ]
                    )
                )));

            resources.push(Box::new(eh.engine_rc()) as Box<Any>);

            let mut fn_control_status: i32 = 0;

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
                            let cont_1 = build_block_call(
                                eh,
                                &entry_fn,
                                &if_builder,
                                if_blk,
                                false
                            );
                            let c1_builder = cervus::engine::Builder::new(&cont_1);
                            c1_builder.append(Action::Branch(&cont_bb));
                        }

                        {
                            let else_builder = cervus::engine::Builder::new(&else_bb);
                            let cont_2 = build_block_call(
                                eh,
                                &entry_fn,
                                &else_builder,
                                else_blk,
                                false
                            );
                            let c2_builder = cervus::engine::Builder::new(&cont_2);
                            c2_builder.append(Action::Branch(&cont_bb));
                        }

                        new_bb = Some(cont_bb);
                    },
                    &mut Operation::Loop(ref mut blk) => {
                        new_bb = Some(build_block_call(
                            eh,
                            &entry_fn,
                            &builder,
                            blk,
                            true
                        ));
                    },
                    &mut Operation::Break => {
                        fn_control_status = 1;
                        break;
                    }
                }
            }

            if new_bb.is_some() {
                bb = std::mem::replace(&mut new_bb, None).unwrap();
            }
            let builder = cervus::engine::Builder::new(&bb);

            builder.append(Action::Return(fn_control_status.into()));
            entry = entry_fn.to_null_handle();
        }

        let ee = cervus::engine::ExecutionEngine::new(m);
        ee.prepare();

        let entry = ee.get_callable_0::<i32>(&entry);

        let mut jit_info = BlockJitInfo {
            resources: resources,
            ee: ee,
            entry: entry
        };

        self.jit_info = Some(jit_info);
        Ok(())
    }
}

fn build_block_call<'a>(
    eh: &engine::EngineHandleImpl,
    f: &'a cervus::engine::Function,
    parent_builder: &cervus::engine::Builder,
    blk: &mut engine::Block,
    is_loop: bool
) -> cervus::engine::BasicBlock<'a> {
    let cont_bb = cervus::engine::BasicBlock::new(f, "");

    let call_block_wrapper_fn = cervus::engine::Value::from(call_block_wrapper as *const c_void as u64)
        .const_int_to_ptr(ValueType::Pointer(Box::new(
            ValueType::Function(
                Box::new(ValueType::Int32),
                vec![
                    ValueType::Pointer(Box::new(ValueType::Void)),
                    ValueType::Pointer(Box::new(ValueType::Void))
                ]
            )
        )));

    let bb = cervus::engine::BasicBlock::new(f, "");
    let builder = cervus::engine::Builder::new(&bb);
    parent_builder.append(Action::Branch(&bb));

    let ret = builder.append(
        Action::Call(
            call_block_wrapper_fn.clone(),
            vec![
                cervus::engine::Value::from(eh as *const engine::EngineHandleImpl as u64).const_int_to_ptr(
                    ValueType::Pointer(Box::new(ValueType::Void))
                ),
                cervus::engine::Value::from(blk as *const engine::Block as u64).const_int_to_ptr(
                    ValueType::Pointer(Box::new(ValueType::Void))
                ),
            ]
        )
    );

    if is_loop {
        builder.append(
            Action::ConditionalBranch(
                builder.append(Action::IntEqual(ret.clone(), (1 as i32).into())), // break
                &cont_bb,
                &bb
            )
        );
    } else {
        let ret_bb = cervus::engine::BasicBlock::new(f, "");
        let ret_builder = cervus::engine::Builder::new(&ret_bb);

        ret_builder.append(Action::Return(ret.clone()));

        builder.append(
            Action::ConditionalBranch(
                builder.append(Action::IntEqual(ret.clone(), (0 as i32).into())), // no control operation
                &cont_bb,
                &ret_bb
            )
        );
    }

    cont_bb
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

extern "C" fn call_block_wrapper(eng: &engine::EngineHandleImpl, blk: &mut engine::Block) -> i32 {
    eng.eval_block(blk)
}
