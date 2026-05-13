pub mod eval;
pub mod value;

pub use eval::{eval, EvalError};
pub use value::Value;