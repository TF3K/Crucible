pub mod eval;
pub mod value;

pub use eval::{EvalError, eval};
pub use value::{DataType, Value};
