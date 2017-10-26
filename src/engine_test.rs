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
        }
    ]
}
    "#;

    let mut eng = engine::Engine::new();
    let blk = eng.load_block(ast).unwrap();
    eng.eval_block(&blk).unwrap();
}
