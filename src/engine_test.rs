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
                    "Plain": {
                        "String": "Hello world"
                    }
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
                "env": [],
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
                    "env": [],
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
                    "env": [],
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
                            "env": [],
                            "stdin": "Inherit",
                            "stdout": "Inherit"
                        }
                    },
                    "EngineBacktrace",
                    {
                        "Print": {
                            "Plain": "Backtrace printed"
                        }
                    },
                    "Break"
                ]
            }
        },
        {
            "CheckEq": [
                {
                    "Plain": {
                        "Integer": 42
                    }
                },
                {
                    "Plain": {
                        "Integer": 42
                    }
                }
            ]
        },
        {
            "AssignGlobal": [
                "cmp_result",
                "LastExitStatus"
            ]
        },
        {
            "AssignGlobal": [
                "test_function",
                {
                    "Plain": {
                        "Function": {
                            "ops": [
                                {
                                    "Print": {
                                        "Plain": "In function!"
                                    }
                                }
                            ]
                        }
                    }
                }
            ]
        },
        {
            "Call": {
                "GlobalVariable": "test_function"
            }
        },
        {
            "Print": {
                "Join": [
                    {
                        "Plain": "Compare result is "
                    },
                    {
                        "GlobalVariable": "cmp_result"
                    }
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
                                "env": [],
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
                                "env": [],
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

    let eng: engine::EngineHandle = engine::Engine::new().into();
    let mut blk = engine::Engine::load_block(ast).unwrap();
    for _ in 0..5 {
        let ret = eng.eval_block(&mut blk);
        if ret != 0 {
            panic!("Pending control status: {}", ret);
        }
    }
}
