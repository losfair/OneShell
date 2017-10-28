use engine;

#[test]
fn test_engine_exec() {
    let ast = r#"
{
    "ops": [
        {
            "Exec": {
                "command": ["ls", "/"],
                "env": {},
                "stdin": "Inherit",
                "stdout": "Inherit"
            }
        },
        {
            "ParallelExec": [
                {
                    "command": ["ls", "/"],
                    "env": {},
                    "stdin": "Inherit",
                    "stdout": {
                        "Pipe": "p1"
                    }
                },
                {
                    "command": ["grep", "etc"],
                    "env": {},
                    "stdin": {
                        "Pipe": "p1"
                    },
                    "stdout": "Inherit"
                }
            ]
        },
        {
            "Loop": {
                "ops": [
                    {
                        "Exec": {
                            "command": ["echo", "In loop"],
                            "env": {},
                            "stdin": "Inherit",
                            "stdout": "Inherit"
                        }
                    },
                    "Break"
                ]
            }
        },
        {
            "IfElse": [
                {
                    "ops": [
                        {
                            "Exec": {
                                "command": ["echo", "Failed"],
                                "env": {},
                                "stdin": "Inherit",
                                "stdout": "Inherit"
                            }
                        }
                    ]
                },
                {
                    "ops": [
                        {
                            "Exec": {
                                "command": ["echo", "OK"],
                                "env": {},
                                "stdin": "Inherit",
                                "stdout": "Inherit"
                            }
                        }
                    ]
                }
            ]
        }
    ]
}
    "#;

    let mut eng: engine::EngineHandle = engine::Engine::new().into();
    let mut blk = eng.borrow_mut().load_block(ast).unwrap();
    for i in 0..5 {
        eng.eval_block(&mut blk);
    }
}
