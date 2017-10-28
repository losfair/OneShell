use engine;

#[test]
fn test_engine_exec() {
    let ast = r#"
{
    "ops": [
        {
            "AssignGlobal": [
                "var1",
                {
                    "String": "Hello world"
                }
            ]
        },
        {
            "Exec": {
                "command": [
                    {
                        "Plain": "ls"
                    },
                    {
                        "Plain": "/"
                    }
                ],
                "env": {},
                "stdin": "Inherit",
                "stdout": "Inherit"
            }
        },
        {
            "ParallelExec": [
                {
                    "command": [
                        {
                            "Plain": "ls"
                        },
                        {
                            "Plain": "/"
                        }
                    ],
                    "env": {},
                    "stdin": "Inherit",
                    "stdout": {
                        "Pipe": "p1"
                    }
                },
                {
                    "command": [
                        {
                            "Plain": "grep"
                        },
                        {
                            "Plain": "etc"
                        }
                    ],
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
                            "command": [
                                {
                                    "Plain": "echo"
                                },
                                {
                                    "GlobalVariable": "var1"
                                }
                            ],
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
                                "command": [
                                    {
                                        "Plain": "echo"
                                    },
                                    {
                                        "Plain": "Failed"
                                    }
                                ],
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
                                "command": [
                                    {
                                        "Plain": "echo"
                                    },
                                    {
                                        "Plain": "OK"
                                    }
                                ],
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
