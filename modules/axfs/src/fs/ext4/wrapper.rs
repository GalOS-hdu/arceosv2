//! lwext4_core 兼容层
//!
//! 提供与 lwext4_rust 兼容的接口，使得可以无缝替换

use alloc::{string::String, vec::Vec};
use core::time::Duration;

use axerrno::LinuxError;
/// 文件属性（兼容 lwext4_rust::FileAttr）
pub use lwext4_core::FileAttr;
// ===== 类型重导出 =====
/// 文件系统配置（兼容 lwext4_rust::FsConfig）
pub use lwext4_core::FsConfig;
/// Inode 类型（兼容 lwext4_rust::InodeType）
pub use lwext4_core::InodeType;
/// SystemHal trait（兼容 lwext4_rust::SystemHal）
pub use lwext4_core::SystemHal;

use super::ArceOsHal;

/// 根目录 inode 编号
pub const EXT4_ROOT_INO: u32 = 2;

// ===== 错误类型兼容层 =====

/// Ext4 错误（兼容 lwext4_rust::Ext4Error）
#[derive(Debug, Clone)]
pub struct Ext4Error {
    pub code: i32,
    pub message: Option<&'static str>,
}

impl Ext4Error {
    pub fn new(code: i32, message: Option<&'static str>) -> Self {
        Self { code, message }
    }

    /// 从 lwext4_core::Error 转换
    fn from_core_error(err: lwext4_core::Error) -> Self {
        use lwext4_core::ErrorKind;

        let code = match err.kind() {
            ErrorKind::NotFound => LinuxError::ENOENT as i32,
            ErrorKind::PermissionDenied => LinuxError::EACCES as i32,
            ErrorKind::AlreadyExists => LinuxError::EEXIST as i32,
            ErrorKind::InvalidInput => LinuxError::EINVAL as i32,
            ErrorKind::Io => LinuxError::EIO as i32,
            ErrorKind::NoSpace => LinuxError::ENOSPC as i32,
            ErrorKind::NotEmpty => LinuxError::ENOTEMPTY as i32,
            ErrorKind::Unsupported => LinuxError::EOPNOTSUPP as i32,
            ErrorKind::Corrupted => LinuxError::EUCLEAN as i32,
            ErrorKind::Busy => LinuxError::EBUSY as i32,
            ErrorKind::InvalidState => LinuxError::EBADFD as i32,
            _ => LinuxError::EIO as i32,
        };

        Self {
            code,
            message: Some(err.message()),
        }
    }
}

/// Ext4 结果类型（兼容 lwext4_rust::Ext4Result）
pub type Ext4Result<T> = Result<T, Ext4Error>;

// ===== 目录条目兼容层 =====

/// 目录条目（兼容 lwext4_rust::DirEntry）
pub struct DirEntry {
    inner: lwext4_core::DirEntry,
}

impl DirEntry {
    pub fn ino(&self) -> u32 {
        self.inner.inode
    }

    pub fn name(&self) -> &[u8] {
        self.inner.name.as_bytes()
    }

    pub fn inode_type(&self) -> InodeType {
        // 将 u8 文件类型转换为 InodeType
        use lwext4_core::dir::write::{EXT4_DE_DIR, EXT4_DE_REG_FILE, EXT4_DE_SYMLINK};

        match self.inner.file_type {
            EXT4_DE_DIR => InodeType::Directory,
            EXT4_DE_REG_FILE => InodeType::RegularFile,
            EXT4_DE_SYMLINK => InodeType::Symlink,
            _ => InodeType::Unknown,
        }
    }
}

// ===== 查找结果 =====

/// 查找结果（兼容 lwext4_rust 的 lookup 返回值）
pub struct LookupResult {
    ino: u32,
    name: Vec<u8>,
    inode_type: InodeType,
}

impl LookupResult {
    pub fn entry(&self) -> DirEntry {
        use lwext4_core::dir::write::{EXT4_DE_DIR, EXT4_DE_REG_FILE, EXT4_DE_SYMLINK};

        let file_type = match self.inode_type {
            InodeType::Directory => EXT4_DE_DIR,
            InodeType::RegularFile => EXT4_DE_REG_FILE,
            InodeType::Symlink => EXT4_DE_SYMLINK,
            _ => 0,
        };

        DirEntry {
            inner: lwext4_core::DirEntry {
                inode: self.ino,
                name: String::from_utf8_lossy(&self.name).into_owned(),
                file_type,
            },
        }
    }
}

// ===== 目录读取器兼容层 =====

/// 目录读取结果（兼容 lwext4_rust 的 read_dir 返回值）
pub struct DirReaderResult {
    entries: Vec<lwext4_core::DirEntry>,
    current_index: usize,
}

impl DirReaderResult {
    /// Returns the current directory entry.
    ///
    /// Used by highlevel file system layer.
    #[allow(dead_code)]
    pub fn entry(&self) -> DirEntry {
        DirEntry {
            inner: self.entries[self.current_index].clone(),
        }
    }

    pub fn current(&self) -> Option<DirEntry> {
        self.entries
            .get(self.current_index)
            .map(|e| DirEntry { inner: e.clone() })
    }

    pub fn step(&mut self) -> Ext4Result<()> {
        if self.current_index < self.entries.len() {
            self.current_index += 1;
        }
        Ok(())
    }

    pub fn offset(&self) -> u64 {
        self.current_index as u64
    }
}

// ===== Inode 引用兼容层 =====

/// Inode 引用包装器（兼容 lwext4_rust 的 with_inode_ref）
pub struct InodeRefWrapper<'a, 'b, D: lwext4_core::BlockDevice> {
    inner: &'a mut lwext4_core::InodeRef<'b, D>,
}

impl<'a, 'b, D: lwext4_core::BlockDevice> InodeRefWrapper<'a, 'b, D> {
    pub fn size(&mut self) -> u64 {
        self.inner.size().unwrap_or(0)
    }

    pub fn mode(&mut self) -> u32 {
        // 通过 with_inode 读取 mode
        self.inner
            .with_inode(|inode| u16::from_le(inode.mode) as u32)
            .unwrap_or(0)
    }

    /// Checks if this inode represents a directory.
    ///
    /// Used by highlevel file system layer.
    #[allow(dead_code)]
    pub fn is_dir(&mut self) -> bool {
        self.inner.is_dir().unwrap_or(false)
    }

    pub fn set_mode(&mut self, mode: u32) {
        let _ = self.inner.set_mode(mode as u16);
    }

    pub fn set_owner(&mut self, uid: u32, gid: u32) {
        let _ = self.inner.set_owner(uid, gid);
    }

    /// Set access time (atime)
    ///
    /// Note: Current lwext4 implementation only supports second precision.
    /// Nanosecond part of the Duration is lost.
    /// TODO: Update when lwext4 supports nanosecond precision.
    pub fn set_atime(&mut self, time: &Duration) {
        let secs = time.as_secs() as u32;
        if time.subsec_nanos() != 0 {
            trace!(
                "set_atime: nanosecond precision lost ({}ns)",
                time.subsec_nanos()
            );
        }
        let _ = self.inner.set_atime(secs);
    }

    /// Set modification time (mtime)
    ///
    /// Note: Current lwext4 implementation only supports second precision.
    /// Nanosecond part of the Duration is lost.
    /// TODO: Update when lwext4 supports nanosecond precision.
    pub fn set_mtime(&mut self, time: &Duration) {
        let secs = time.as_secs() as u32;
        if time.subsec_nanos() != 0 {
            trace!(
                "set_mtime: nanosecond precision lost ({}ns)",
                time.subsec_nanos()
            );
        }
        let _ = self.inner.set_mtime(secs);
    }

    pub fn update_ctime(&mut self) {
        if let Some(now) = ArceOsHal::now() {
            let _ = self.inner.set_ctime(now.as_secs() as u32);
        }
    }
}

// ===== 文件系统统计信息 =====

/// 文件系统统计信息（兼容 lwext4_rust）
#[derive(Debug, Clone, Default)]
pub struct FsStat {
    pub block_size: u32,
    pub blocks_count: u64,
    pub free_blocks_count: u64,
    pub inodes_count: u32,
    pub free_inodes_count: u32,
}

// ===== 主文件系统封装 =====

/// Ext4 文件系统（兼容 lwext4_rust::Ext4Filesystem）
pub struct Ext4Filesystem<H: SystemHal, D: lwext4_core::BlockDevice> {
    inner: lwext4_core::Ext4FileSystem<D>,
    _phantom: core::marker::PhantomData<H>,
}

impl<H: SystemHal, D: lwext4_core::BlockDevice> Ext4Filesystem<H, D> {
    /// 创建新的文件系统实例（兼容 lwext4_rust）
    pub fn new(device: D, _config: FsConfig) -> Ext4Result<Self> {
        // 将设备包装为 BlockDev
        // 🚀 性能优化：启用块缓存，1024个块（4MB缓存，假设4KB块大小）
        //
        // 缓存大小实测结果：
        // - 1024块(4MB): 15.7 MB/s (c1)
        // - 2048块(8MB): 14.9 MB/s (c2) ← 反而下降！
        //
        // 原因分析：
        // - LRU管理开销随缓存增大而增长
        // - HashMap冲突和rehash开销
        // - 内存分配压力
        //
        // 结论：保持1024块
        // - 与lwext4(256块)相比已是4倍
        // - dd测试中缓存命中率不是瓶颈（顺序写入不依赖缓存）
        // - 性能瓶颈在数据拷贝和InodeRef获取
        let bdev = lwext4_core::BlockDev::new_with_cache(device, 1024)
            .map_err(Ext4Error::from_core_error)?;
        // let bdev = lwext4_core::BlockDev::new(device)
        //     .map_err(Ext4Error::from_core_error)?;

        let inner = lwext4_core::Ext4FileSystem::mount(bdev).map_err(Ext4Error::from_core_error)?;

        Ok(Self {
            inner,
            _phantom: core::marker::PhantomData,
        })
    }

    /// 获取文件系统统计信息
    pub fn stat(&mut self) -> Ext4Result<FsStat> {
        let stats = self.inner.stats().map_err(Ext4Error::from_core_error)?;

        Ok(FsStat {
            block_size: stats.block_size,
            blocks_count: stats.blocks_total,
            free_blocks_count: stats.blocks_free,
            inodes_count: stats.inodes_total,
            free_inodes_count: stats.inodes_free,
        })
    }

    /// 刷新文件系统
    pub fn flush(&mut self) -> Ext4Result<()> {
        // 刷新块缓存中的所有脏数据到磁盘
        self.inner.flush().map_err(Ext4Error::from_core_error)
    }

    /// 查找目录项
    ///
    /// 注意：这个方法返回 LookupResult 而不是单个 DirEntry
    /// 这是为了兼容 lwext4_rust 的 API
    pub fn lookup(&mut self, dir_ino: u32, name: &str) -> Ext4Result<LookupResult> {
        let child_ino = self
            .inner
            .lookup_in_dir(dir_ino, name)
            .map_err(Ext4Error::from_core_error)?;

        // 获取文件属性来构造完整的 DirEntry
        let metadata = self
            .inner
            .get_inode_attr(child_ino)
            .map_err(Ext4Error::from_core_error)?;

        // 将 FileType 转换为 InodeType
        let inode_type = match metadata.file_type {
            lwext4_core::FileType::Directory => InodeType::Directory,
            lwext4_core::FileType::RegularFile => InodeType::RegularFile,
            lwext4_core::FileType::Symlink => InodeType::Symlink,
            _ => InodeType::Unknown,
        };

        Ok(LookupResult {
            ino: child_ino,
            name: name.as_bytes().to_vec(),
            inode_type,
        })
    }

    /// 获取文件属性
    pub fn get_attr(&mut self, ino: u32, attr: &mut FileAttr) -> Ext4Result<()> {
        let metadata = self
            .inner
            .get_inode_attr(ino)
            .map_err(Ext4Error::from_core_error)?;

        // 将 FileMetadata 转换为 FileAttr
        let node_type = match metadata.file_type {
            lwext4_core::FileType::Directory => InodeType::Directory,
            lwext4_core::FileType::RegularFile => InodeType::RegularFile,
            lwext4_core::FileType::Symlink => InodeType::Symlink,
            lwext4_core::FileType::CharDevice => InodeType::CharacterDevice,
            lwext4_core::FileType::BlockDevice => InodeType::BlockDevice,
            lwext4_core::FileType::Fifo => InodeType::Fifo,
            lwext4_core::FileType::Socket => InodeType::Socket,
            _ => InodeType::Unknown,
        };

        *attr = FileAttr {
            device: 0, // TODO: 获取设备号
            nlink: metadata.links_count as u32,
            mode: metadata.permissions as u32,
            node_type,
            uid: metadata.uid,
            gid: metadata.gid,
            size: metadata.size,
            block_size: 4096, // TODO: 从文件系统获取
            blocks: metadata.blocks_count,
            atime: metadata.atime as u64,
            mtime: metadata.mtime as u64,
            ctime: metadata.ctime as u64,
        };
        Ok(())
    }

    /// 读取文件数据
    pub fn read_at(&mut self, ino: u32, buf: &mut [u8], offset: u64) -> Ext4Result<usize> {
        if buf.len() > 0 {
            info!(
                "[ext4] READ: ino={}, len={}, offset={}",
                ino,
                buf.len(),
                offset
            );
        }

        let result = self
            .inner
            .read_at_inode(ino, buf, offset)
            .map_err(Ext4Error::from_core_error);

        match result {
            Ok(n) => {
                info!("[ext4] READ SUCCESS: ino={}, read={}", ino, n);
                Ok(n)
            }
            Err(e) => {
                // 检查是否是"块不存在"错误（稀疏文件的空洞）
                // 对于稀疏文件，读取未分配的块应该返回零，而不是错误
                if e.code == 2 && e.message == Some("Logical block not found in extent tree") {
                    // 这是稀疏文件的空洞，填充零并返回成功
                    info!(
                        "[ext4] READ sparse hole: ino={}, len={}, offset={}, returning zeros",
                        ino,
                        buf.len(),
                        offset
                    );
                    buf.fill(0);
                    Ok(buf.len())
                } else {
                    // 其他错误正常报告
                    warn!(
                        "[ext4] READ FAILED: ino={}, len={}, offset={}, error={:?}",
                        ino,
                        buf.len(),
                        offset,
                        e
                    );
                    Err(e)
                }
            }
        }
    }

    /// 写入文件数据
    pub fn write_at(&mut self, ino: u32, buf: &[u8], offset: u64) -> Ext4Result<usize> {
        // 记录所有写入操作（降低阈值以捕获小文件写入）
        if buf.len() > 0 {
            info!(
                "[ext4] WRITE: ino={}, len={}, offset={}",
                ino,
                buf.len(),
                offset
            );
        }

        // 🚀 性能优化：使用批量写入接口，避免重复获取InodeRef
        let result = self
            .inner
            .write_at_inode_batch(ino, buf, offset)
            .map_err(Ext4Error::from_core_error);

        match &result {
            Ok(written) => info!("[ext4] WRITE SUCCESS: ino={}, written={}", ino, written),
            Err(e) => warn!(
                "[ext4] WRITE FAILED: ino={}, len={}, offset={}, error={:?}",
                ino,
                buf.len(),
                offset,
                e
            ),
        }

        result
    }

    /// 设置文件大小
    pub fn set_len(&mut self, ino: u32, len: u64) -> Ext4Result<()> {
        info!("[ext4] SET_LEN: ino={}, new_len={}", ino, len);

        // 获取当前文件大小
        let current_size = self
            .inner
            .with_inode_ref(ino, |inode| inode.size())
            .map_err(Ext4Error::from_core_error)?;

        // 统一使用 truncate_file 处理所有大小变更
        // truncate_file 已经正确实现了：
        // 1. 缩小文件：释放不需要的块
        // 2. 扩展文件：只更新 i_size（稀疏文件），不分配块
        if len != current_size {
            info!(
                "[ext4] SET_LEN: ino={}, {} from {} to {} (sparse)",
                ino,
                if len > current_size {
                    "expanding"
                } else {
                    "shrinking"
                },
                current_size,
                len
            );

            self.inner
                .truncate_file(ino, len)
                .map_err(Ext4Error::from_core_error)
        } else {
            // 大小不变：什么都不做
            Ok(())
        }
    }

    /// 设置符号链接
    pub fn set_symlink(&mut self, ino: u32, target: &[u8]) -> Ext4Result<()> {
        if target.len() < 60 {
            // 快速符号链接：存储在 inode.blocks 中
            self.inner
                .with_inode_ref(ino, |inode_ref| {
                    inode_ref.set_size(target.len() as u64)?;
                    inode_ref.with_inode_mut(|inode| {
                        let block_slice = unsafe {
                            core::slice::from_raw_parts_mut(
                                inode.blocks.as_mut_ptr() as *mut u8,
                                60,
                            )
                        };
                        block_slice[..target.len()].copy_from_slice(target);
                    })?;
                    Ok(())
                })
                .map_err(Ext4Error::from_core_error)
        } else {
            // 慢速符号链接：写入数据块
            // write_at_inode_batch 会自动更新文件大小，无需手动调用 truncate_file
            let written = self
                .inner
                .write_at_inode_batch(ino, target, 0)
                .map_err(Ext4Error::from_core_error)?;

            if written != target.len() {
                return Err(Ext4Error::new(
                    LinuxError::EIO as i32,
                    Some("Failed to write symlink target"),
                ));
            }
            Ok(())
        }
    }

    /// 读取符号链接的目标路径
    ///
    /// Used by highlevel file system layer for symlink operations.
    #[allow(dead_code)]
    pub fn readlink(&mut self, ino: u32) -> Ext4Result<Vec<u8>> {
        use lwext4_core::consts::*;

        self.inner
            .with_inode_ref(ino, |inode_ref| {
                // 验证是符号链接
                let mode = inode_ref.with_inode(|inode| u16::from_le(inode.mode))?;
                if (mode & EXT4_INODE_MODE_TYPE_MASK) != EXT4_INODE_MODE_SOFTLINK {
                    return Err(lwext4_core::Error::new(
                        lwext4_core::ErrorKind::InvalidInput,
                        "Not a symlink",
                    ));
                }

                let size = inode_ref.size()? as usize;
                if size == 0 {
                    return Ok(Vec::new());
                }

                // 读取目标路径
                let result = if size < 60 {
                    // 快速符号链接：从 inode.blocks 读取
                    inode_ref.with_inode(|inode| {
                        let block_slice = unsafe {
                            core::slice::from_raw_parts(inode.blocks.as_ptr() as *const u8, size)
                        };
                        block_slice.to_vec()
                    })?
                } else {
                    // 慢速符号链接：从数据块读取
                    // 不能使用 read_extent_file（会拒绝符号链接），需要手动读取
                    let block_addr = inode_ref.get_inode_dblk_idx(0, false)?;
                    if block_addr == 0 {
                        return Err(lwext4_core::Error::new(
                            lwext4_core::ErrorKind::NotFound,
                            "Symlink data block not found",
                        ));
                    }

                    // 获取 block_size
                    let block_size = inode_ref.superblock().block_size() as usize;
                    let mut block_buf = alloc::vec![0u8; block_size];

                    // 直接读取块
                    inode_ref.bdev().read_block(block_addr, &mut block_buf)?;
                    block_buf[..size].to_vec()
                };
                Ok(result)
            })
            .map_err(Ext4Error::from_core_error)
    }

    /// 读取目录
    pub fn read_dir(&mut self, dir_ino: u32, offset: u64) -> Ext4Result<DirReaderResult> {
        // 使用 read_dir_from_inode API
        let entries = self
            .inner
            .read_dir_from_inode(dir_ino)
            .map_err(Ext4Error::from_core_error)?;

        Ok(DirReaderResult {
            entries,
            current_index: offset as usize,
        })
    }

    /// 创建文件或目录
    pub fn create(
        &mut self,
        parent_ino: u32,
        name: &str,
        inode_type: InodeType,
        mode: u32,
    ) -> Ext4Result<u32> {
        use lwext4_core::dir::write::{EXT4_DE_DIR, EXT4_DE_REG_FILE, EXT4_DE_SYMLINK};

        let file_type = match inode_type {
            InodeType::RegularFile => EXT4_DE_REG_FILE,
            InodeType::Directory => EXT4_DE_DIR,
            InodeType::Symlink => EXT4_DE_SYMLINK,
            _ => {
                return Err(Ext4Error::new(
                    LinuxError::EOPNOTSUPP as i32,
                    Some("Unsupported inode type"),
                ));
            }
        };

        // 如果 mode 的权限位为 0，应用默认权限
        // 这确保文件始终有合理的权限，即使应用程序传入 mode=0
        // 这是标准的 Unix 行为，防止创建完全无权限的文件
        let effective_mode = if mode & 0o777 == 0 {
            match inode_type {
                InodeType::Directory => 0o755, // rwxr-xr-x
                _ => 0o644,                    // rw-r--r--
            }
        } else {
            mode & 0o777 // 只保留权限位
        };

        info!(
            "[ext4] CREATE: parent_ino={}, name={:?}, type={:?}, mode={:#o}, effective_mode={:#o}",
            parent_ino, name, inode_type, mode, effective_mode
        );

        let result = self
            .inner
            .create_in_dir(parent_ino, name, file_type, effective_mode as u16)
            .map_err(Ext4Error::from_core_error);

        match &result {
            Ok(ino) => {
                info!("[ext4] CREATE SUCCESS: new_ino={}", ino);
                // 设置新创建文件的时间戳为当前时间
                let now = core::time::Duration::from_secs(axhal::time::wall_time().as_secs());
                if let Err(e) = self.with_inode_ref(*ino, |inode| {
                    inode.set_atime(&now);
                    inode.set_mtime(&now);
                    inode.update_ctime();
                    Ok(())
                }) {
                    warn!(
                        "[ext4] Failed to set timestamps for new inode {}: {:?}",
                        ino, e
                    );
                }
            }
            Err(e) => warn!("[ext4] CREATE FAILED: error={:?}", e),
        }

        result
    }

    /// 删除文件
    pub fn unlink(&mut self, parent_ino: u32, name: &str) -> Ext4Result<()> {
        self.inner
            .unlink_from_dir(parent_ino, name)
            .map(|_| ())
            .map_err(Ext4Error::from_core_error)
    }

    /// 重命名
    pub fn rename(
        &mut self,
        src_parent: u32,
        src_name: &str,
        dst_parent: u32,
        dst_name: &str,
    ) -> Ext4Result<()> {
        self.inner
            .rename_inode(src_parent, src_name, dst_parent, dst_name)
            .map_err(Ext4Error::from_core_error)
    }

    /// 创建硬链接
    pub fn link(&mut self, parent_ino: u32, name: &str, target_ino: u32) -> Ext4Result<()> {
        self.inner
            .link_inode(parent_ino, name, target_ino)
            .map_err(Ext4Error::from_core_error)
    }

    /// 操作 inode 引用
    pub fn with_inode_ref<F, R>(&mut self, ino: u32, f: F) -> Ext4Result<R>
    where
        F: for<'a, 'b> FnOnce(&'a mut InodeRefWrapper<'a, 'b, D>) -> Ext4Result<R>,
    {
        self.inner
            .with_inode_ref(ino, |inode_ref| {
                let mut wrapper = InodeRefWrapper { inner: inode_ref };
                f(&mut wrapper).map_err(|e| {
                    lwext4_core::Error::new(
                        lwext4_core::ErrorKind::Io,
                        e.message.unwrap_or("Error"),
                    )
                })
            })
            .map_err(Ext4Error::from_core_error)
    }

    /// Deferred deletion: 当VFS层释放最后一个对inode的引用时调用
    /// 如果 i_nlink == 0，则释放inode的所有资源
    ///
    /// Used by VFS layer for inode cleanup.
    #[allow(dead_code)]
    pub fn drop_inode(&mut self, ino: u32) -> Ext4Result<()> {
        self.inner
            .drop_inode(ino)
            .map_err(Ext4Error::from_core_error)
    }

    /// 列出扩展属性
    ///
    /// 返回所有扩展属性名称（以 null 结尾的字符串列表）
    pub fn listxattr(&mut self, ino: u32, buffer: &mut [u8]) -> Ext4Result<usize> {
        self.inner
            .with_inode_ref(ino, |inode_ref| lwext4_core::xattr::list(inode_ref, buffer))
            .map_err(Ext4Error::from_core_error)
    }

    /// 获取扩展属性值
    ///
    /// # 参数
    /// * `ino` - inode 编号
    /// * `name` - 属性名（含命名空间前缀，如 "user.comment"）
    /// * `buffer` - 输出缓冲区
    ///
    /// 返回属性值的长度
    pub fn getxattr(&mut self, ino: u32, name: &str, buffer: &mut [u8]) -> Ext4Result<usize> {
        self.inner
            .with_inode_ref(ino, |inode_ref| {
                lwext4_core::xattr::get(inode_ref, name, buffer)
            })
            .map_err(Ext4Error::from_core_error)
    }

    /// 设置扩展属性
    ///
    /// # 参数
    /// * `ino` - inode 编号
    /// * `name` - 属性名（含命名空间前缀）
    /// * `value` - 属性值
    pub fn setxattr(&mut self, ino: u32, name: &str, value: &[u8]) -> Ext4Result<()> {
        self.inner
            .with_inode_ref(ino, |inode_ref| {
                lwext4_core::xattr::set(inode_ref, name, value)
            })
            .map_err(Ext4Error::from_core_error)
    }

    /// 删除扩展属性
    ///
    /// # 参数
    /// * `ino` - inode 编号
    /// * `name` - 要删除的属性名
    pub fn removexattr(&mut self, ino: u32, name: &str) -> Ext4Result<()> {
        self.inner
            .with_inode_ref(ino, |inode_ref| lwext4_core::xattr::remove(inode_ref, name))
            .map_err(Ext4Error::from_core_error)
    }
}
