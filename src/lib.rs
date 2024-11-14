use linux::LinuxError;

#[cfg(target_os = "linux")]
mod linux;

pub struct ProtocolHandler {
    pub name: String,
    pub protocol_name: String,
}

impl ProtocolHandler {
    #[cfg(target_os = "linux")]
    pub fn register(&self) -> Result<(), LinuxError> {
        linux::register(&self.name, &self.protocol_name)
    }

    #[cfg(target_os = "linux")]
    pub fn unregister(&self) -> Result<(), LinuxError> {
        linux::unregister(&self.name)
    }
}
