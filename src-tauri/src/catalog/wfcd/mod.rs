//! WFCD 上游：常量 URL、HTTP fetch、轻量 parse / 家族规则。
//!
//! - 写 catalog 事务见 [`crate::catalog::import`]（机体）与 [`crate::catalog::weapons`]（武器）
//! - 本模块不打开 sqlite

mod constants;
mod fetch;
mod parse;

pub use constants::*;
pub use fetch::*;
pub use parse::*;
