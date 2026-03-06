use axerrno::LinuxError;
use axfs_ng_vfs::{NodeType, VfsError};

use super::{ArceOsHal, Ext4CoreDisk, wrapper};

// 使用 wrapper 层的类型，保持 API 兼容性
pub type Ext4Error = wrapper::Ext4Error;
pub type InodeType = wrapper::InodeType;

// 使用 lwext4_core 通过 wrapper 层
pub type LwExt4Filesystem = wrapper::Ext4Filesystem<ArceOsHal, Ext4CoreDisk>;

pub fn into_vfs_err(err: Ext4Error) -> VfsError {
    // 只记录非 NotFound 的错误，因为 NotFound 是正常的文件系统操作
    // （例如：程序检查文件是否存在）
    let linux_error = LinuxError::try_from(err.code).unwrap_or(LinuxError::EIO);

    if linux_error != LinuxError::ENOENT {
        warn!("[ext4] Error occurred: code={} ({:?}), message={:?}",
              err.code, linux_error, err.message);
    }

    let vfs_err = VfsError::from(linux_error).canonicalize();

    if linux_error != LinuxError::ENOENT {
        warn!("[ext4] Converted to VfsError: {:?}", vfs_err);
    }

    vfs_err
}

pub fn into_vfs_type(ty: InodeType) -> NodeType {
    match ty {
        InodeType::RegularFile => NodeType::RegularFile,
        InodeType::Directory => NodeType::Directory,
        InodeType::CharacterDevice => NodeType::CharacterDevice,
        InodeType::BlockDevice => NodeType::BlockDevice,
        InodeType::Fifo => NodeType::Fifo,
        InodeType::Socket => NodeType::Socket,
        InodeType::Symlink => NodeType::Symlink,
        InodeType::Unknown => NodeType::Unknown,
    }
}
