mod fs;
mod inode;
mod util;

// lwext4_core 支持
mod adapter;
mod hal;
pub mod wrapper;

pub use adapter::Ext4CoreDisk;
#[allow(unused_imports)]
use axdriver::{AxBlockDevice, prelude::BlockDriverOps};
pub use fs::*;
// 导出 lwext4_core 相关类型
pub use hal::ArceOsHal;
pub use inode::*;
// 导出 wrapper
#[allow(unused)]
pub use wrapper as lwext4_core_compat;
