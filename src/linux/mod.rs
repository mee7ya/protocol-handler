use std::{
    env::{self, current_exe, var},
    fs::{File, OpenOptions},
    io::{self, Read},
    os::unix::fs::FileExt,
    str::Lines,
    usize,
};

use indexmap::IndexMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LinuxError {
    #[error("{0}")]
    ParseError(String),
    #[error("{0}")]
    IoError(#[from] io::Error),
    #[error("{0}")]
    EnvError(#[from] env::VarError),
}

#[derive(Debug, PartialEq)]
struct DesktopEntry {
    data: IndexMap<String, String>,
}

impl TryFrom<String> for DesktopEntry {
    type Error = LinuxError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let mut lines: Lines = s.lines();
        match lines.next() {
            Some(val) => {
                if val != "[Desktop Entry]" {
                    return Err(LinuxError::ParseError("Not a desktop entry".to_string()));
                }
            }
            None => {}
        }

        let mut data: IndexMap<String, String> = IndexMap::new();
        for line in lines {
            let split: Vec<&str> = line.split('=').collect();
            if split.len() != 2 {
                return Err(LinuxError::ParseError("Invalid field format".to_string()));
            }
            data.insert(split[0].to_string(), split[1].to_string());
        }

        let exe = current_exe()?.to_string_lossy().to_string();
        data.entry("Exec".to_string()).or_insert(exe);

        Ok(DesktopEntry { data })
    }
}

impl ToString for DesktopEntry {
    fn to_string(&self) -> String {
        format!(
            "[Desktop Entry]\n{}",
            self.data
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<String>>()
                .join("\n"),
        )
    }
}

impl DesktopEntry {
    pub fn insert_scheme_handler(&mut self, entry: String) {
        match self.data.get_mut("MimeType") {
            Some(val) => {
                let mut split: Vec<&str> = val.split(';').filter(|x| !x.is_empty()).collect();
                let scheme_handler_pos: usize = split
                    .iter()
                    .position(|x| x.starts_with("x-scheme-handler/"))
                    .unwrap_or(usize::MAX);
                if scheme_handler_pos != usize::MAX {
                    split[scheme_handler_pos] = &entry
                } else {
                    split.push(&entry);
                }
                *val = split.join(";");
            }
            None => {
                self.data.insert("MimeType".to_string(), entry);
            }
        }
    }
}

fn get_file(name: &String) -> Result<File, LinuxError> {
    let home: String = var("HOME")?;
    let path: String = format!("{home}/.local/share/applications/{}.desktop", name);

    Ok(OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?)
}

pub fn register(name: &String, protocol_name: &String) -> Result<(), LinuxError> {
    let mut file = get_file(name)?;
    let mut content = String::new();

    file.read_to_string(&mut content)?;
    let mut de: DesktopEntry = DesktopEntry::try_from(content)?;

    de.insert_scheme_handler(format!("x-scheme-handler/{protocol_name}"));

    file.set_len(0)?;
    file.write_at(de.to_string().as_bytes(), 0)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_entry() {
        let content: String = "[Not Desktop Entry]".to_string();
        assert!(
            DesktopEntry::try_from(content).is_err(),
            "Not a desktop entry"
        )
    }

    #[test]
    fn test_invalid_fields() {
        let content: String = "[Desktop Entry]\nnotvalid".to_string();
        assert!(
            DesktopEntry::try_from(content).is_err(),
            "Invalid field format"
        )
    }

    #[test]
    fn test_valid() {
        let content: String = "[Desktop Entry]\nfield1=val1\nfield2=val2".to_string();
        let de = DesktopEntry::try_from(content);
        assert!(de.is_ok());

        let de = de.unwrap();
        assert!(de.data.contains_key("field1"));
        assert!(de.data.contains_key("field2"));
    }

    #[test]
    fn test_to_string() {
        let de: DesktopEntry = DesktopEntry {
            data: IndexMap::from([
                ("field1".to_string(), "val1".to_string()),
                ("field2".to_string(), "val2".to_string()),
            ]),
        };
        assert_eq!(de.to_string(), "[Desktop Entry]\nfield1=val1\nfield2=val2")
    }

    #[test]
    fn test_insert_scheme_handler() {
        let content: String = "[Desktop Entry]\nfield1=val1\nfield2=val2".to_string();
        let mut de = DesktopEntry::try_from(content).unwrap();
        de.insert_scheme_handler("x-scheme-handler/app".to_string());
        assert_eq!(
            de.data.get("MimeType"),
            Some(&"x-scheme-handler/app".to_string())
        );
    }

    #[test]
    fn test_insert_scheme_handler_has_mime_type() {
        let content: String =
            "[Desktop Entry]\nfield1=val1\nfield2=val2\nMimeType=application/cdf".to_string();
        let mut de = DesktopEntry::try_from(content).unwrap();
        de.insert_scheme_handler("x-scheme-handler/app".to_string());
        assert_eq!(
            de.data.get("MimeType"),
            Some(&"application/cdf;x-scheme-handler/app".to_string())
        );
    }

    #[test]
    fn test_insert_scheme_handler_replace() {
        let content: String =
            "[Desktop Entry]\nfield1=val1\nfield2=val2\nMimeType=x-scheme-handler/app".to_string();
        let mut de = DesktopEntry::try_from(content).unwrap();
        de.insert_scheme_handler("x-scheme-handler/app2".to_string());
        assert_eq!(
            de.data.get("MimeType"),
            Some(&"x-scheme-handler/app2".to_string())
        );
    }
}
