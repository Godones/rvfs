use onlyerror::Error;

#[derive(Debug,Error)]
pub enum VfsError{
    #[error("Permission denied")]
    PermissionDenied,
}