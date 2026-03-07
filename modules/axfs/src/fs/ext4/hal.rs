//! Hardware Abstraction Layer for lwext4_core
//!
//! 为 lwext4_core 提供系统时间接口

use core::time::Duration;

use lwext4_core::SystemHal;

/// ArceOS Hardware Abstraction Layer implementation
pub struct ArceOsHal;

impl SystemHal for ArceOsHal {
    fn now() -> Option<Duration> {
        // 集成 ArceOS 的时间接口
        #[cfg(feature = "times")]
        {
            Some(axhal::time::wall_time())
        }
        #[cfg(not(feature = "times"))]
        {
            None
        }
    }
}
