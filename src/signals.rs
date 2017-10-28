use std;
use std::error::Error;
use engine;

#[derive(Debug, Clone)]
pub struct Continue {
}

#[derive(Debug, Clone)]
pub struct Break {
}

impl Error for Continue {
    fn description(&self) -> &str {
        return "Continue signal not handled";
    }
}

impl std::fmt::Display for Continue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Continue {
    pub fn new() -> Continue {
        Continue {
        }
    }
}

impl Error for Break {
    fn description(&self) -> &str {
        return "Break signal not handled";
    }
}

impl std::fmt::Display for Break {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Break {
    pub fn new() -> Break {
        Break {
        }
    }
}
