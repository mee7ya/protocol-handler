use std::error::Error;

#[cfg(target_os = "linux")]
mod linux;

pub struct ProtocolHandler {
    pub name: String,
    pub protocol_name: String,
}

impl ProtocolHandler {
    #[cfg(target_os = "linux")]
    pub fn register(&self) -> Result<(), Box<dyn Error>> {
        linux::register(&self.name, &self.protocol_name)
    }
}
