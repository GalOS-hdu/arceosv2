//! Block device adapters for lwext4_core
//!
//! 提供 AxBlockDevice 到 lwext4_core::BlockDevice 的适配器

use axdriver::AxBlockDevice;

/// Adapter for AxBlockDevice to work with lwext4_core
pub struct Ext4CoreDisk {
    inner: AxBlockDevice,
}

impl Ext4CoreDisk {
    /// Create a new adapter from AxBlockDevice
    pub fn new(dev: AxBlockDevice) -> Self {
        Self { inner: dev }
    }
}

impl lwext4_core::BlockDevice for Ext4CoreDisk {
    fn block_size(&self) -> u32 {
        // ext4 文件系统块大小（通常为 4096 字节）
        // 注：这个值会在挂载时从superblock读取并验证
        4096
    }

    fn sector_size(&self) -> u32 {
        // 物理扇区大小
        use axdriver::prelude::BlockDriverOps;
        self.inner.block_size() as u32
    }

    fn total_blocks(&self) -> u64 {
        // 总块数（以文件系统块为单位）
        use axdriver::prelude::BlockDriverOps;
        let device_block_size = self.inner.block_size() as u64;
        let fs_block_size = 4096u64;
        let device_blocks = self.inner.num_blocks();
        // 转换：设备块数 * 设备块大小 / 文件系统块大小
        (device_blocks * device_block_size) / fs_block_size
    }

    fn read_blocks(&mut self, lba: u64, count: u32, buf: &mut [u8]) -> lwext4_core::Result<usize> {
        use axdriver::prelude::BlockDriverOps;

        // 注意：BlockDevice trait 的 read_blocks 参数是以**扇区**为单位
        // lba 和 count 都是扇区号和扇区数，不是文件系统块
        let sector_size = self.sector_size() as usize;
        let expected_size = count as usize * sector_size;

        if buf.len() < expected_size {
            warn!("[adapter] Buffer too small: buf.len()={}, expected={}, lba={}, count={}",
                  buf.len(), expected_size, lba, count);
            return Err(lwext4_core::Error::new(
                lwext4_core::ErrorKind::InvalidInput,
                "Buffer too small"
            ));
        }

        // 检查是否超出设备范围
        let device_total_sectors = self.inner.num_blocks();
        if lba + count as u64 > device_total_sectors as u64 {
            warn!("[adapter] Read out of bounds: lba={}, count={}, device_total={}",
                  lba, count, device_total_sectors);
            return Err(lwext4_core::Error::new(
                lwext4_core::ErrorKind::Io,
                "Read out of bounds"
            ));
        }

        // 直接读取扇区（AxBlockDevice的block就是扇区）
        self.inner
            .read_block(lba, &mut buf[..expected_size])
            .map_err(|e| {
                warn!("[adapter] Failed to read: lba={}, count={}, error={:?}", lba, count, e);
                lwext4_core::Error::new(lwext4_core::ErrorKind::Io, "Block read failed")
            })?;

        Ok(expected_size)
    }

    fn write_blocks(&mut self, lba: u64, count: u32, buf: &[u8]) -> lwext4_core::Result<usize> {
        use axdriver::prelude::BlockDriverOps;

        // 注意：BlockDevice trait 的 write_blocks 参数是以**扇区**为单位
        let sector_size = self.sector_size() as usize;
        let expected_size = count as usize * sector_size;

        if buf.len() < expected_size {
            warn!("[adapter] Buffer too small: buf.len()={}, expected={}, lba={}, count={}",
                  buf.len(), expected_size, lba, count);
            return Err(lwext4_core::Error::new(
                lwext4_core::ErrorKind::InvalidInput,
                "Buffer too small"
            ));
        }

        // 检查是否超出设备范围
        let device_total_sectors = self.inner.num_blocks();
        if lba + count as u64 > device_total_sectors as u64 {
            warn!("[adapter] Write out of bounds: lba={}, count={}, device_total={}",
                  lba, count, device_total_sectors);
            return Err(lwext4_core::Error::new(
                lwext4_core::ErrorKind::Io,
                "Write out of bounds"
            ));
        }

        // 直接写入扇区
        self.inner
            .write_block(lba, &buf[..expected_size])
            .map_err(|e| {
                warn!("[adapter] Failed to write: lba={}, count={}, error={:?}", lba, count, e);
                lwext4_core::Error::new(lwext4_core::ErrorKind::Io, "Block write failed")
            })?;

        Ok(expected_size)
    }

    fn flush(&mut self) -> lwext4_core::Result<()> {
        use axdriver::prelude::BlockDriverOps;

        self.inner
            .flush()
            .map_err(|e| {
                warn!("[adapter] Failed to flush: error={:?}", e);
                lwext4_core::Error::new(lwext4_core::ErrorKind::Io, "Block flush failed")
            })?;

        Ok(())
    }
}
