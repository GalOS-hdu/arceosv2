mod fs;
mod inode;
mod util;

// lwext4_core 支持
mod hal;
mod adapter;
pub mod wrapper;

#[allow(unused_imports)]
use axdriver::{AxBlockDevice, prelude::BlockDriverOps};
pub use fs::*;
pub use inode::*;

// 导出 lwext4_core 相关类型
pub use hal::ArceOsHal;
pub use adapter::Ext4CoreDisk;

// 导出 wrapper
#[allow(unused)]
pub use wrapper as lwext4_core_compat;
