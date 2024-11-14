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

#[derive(Debug)]
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

        let mut exe = current_exe()?.to_string_lossy().to_string();
        exe.push_str(" %u");

        data.entry("Exec".to_string()).or_insert(exe);

        Ok(DesktopEntry { data })
    }
}

impl TryFrom<&mut File> for DesktopEntry {
    type Error = LinuxError;

    fn try_from(value: &mut File) -> Result<Self, Self::Error> {
        let mut content = String::new();
        value.read_to_string(&mut content)?;

        Self::try_from(content)
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
    fn get_mime_types(&self) -> Option<Vec<&str>> {
        match self.data.get("MimeType") {
            Some(val) => Some(val.split(';').filter(|x| !x.is_empty()).collect()),
            None => None,
        }
    }

    fn find_mime_type(&self, split: &Vec<&str>, starts_with: &str) -> Option<usize> {
        split.iter().position(|x| x.starts_with(starts_with))
    }

    pub fn insert_scheme_handler(&mut self, entry: String) {
        match self.get_mime_types() {
            Some(mut split) => {
                match self.find_mime_type(&split, "x-scheme-handler/") {
                    Some(position) => split[position] = &entry,
                    None => split.push(&entry),
                }
                self.data.insert("MimeType".to_string(), split.join(";"));
            }
            None => {
                self.data.insert("MimeType".to_string(), entry);
            }
        }
    }

    pub fn delete_scheme_handler(&mut self) {
        if let Some(mut split) = self.get_mime_types() {
            if let Some(position) = self.find_mime_type(&split, "x-scheme-handler/") {
                split.remove(position);
                if !split.is_empty() {
                    self.data.insert("MimeType".to_string(), split.join(";"));
                } else {
                    self.data.shift_remove("MimeType");
                }
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
    let mut de: DesktopEntry = DesktopEntry::try_from(&mut file)?;

    de.insert_scheme_handler(format!("x-scheme-handler/{protocol_name}"));

    file.set_len(0)?;
    file.write_at(de.to_string().as_bytes(), 0)?;
    Ok(())
}

pub fn unregister(name: &String) -> Result<(), LinuxError> {
    let mut file = get_file(name)?;
    let mut de: DesktopEntry = DesktopEntry::try_from(&mut file)?;

    de.delete_scheme_handler();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexmap;

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
            data: indexmap! {
                "field1".to_string() => "val1".to_string(),
                "field2".to_string() => "val2".to_string(),
            },
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

    #[test]
    fn test_delete_scheme_handler_full() {
        let content: String =
            "[Desktop Entry]\nfield1=val1\nfield2=val2\nMimeType=x-scheme-handler/app".to_string();
        let mut de = DesktopEntry::try_from(content).unwrap();
        de.delete_scheme_handler();
        assert!(!de.data.contains_key("MimeType"));
    }

    #[test]
    fn test_delete_scheme_handler_partial() {
        let content: String =
            "[Desktop Entry]\nfield1=val1\nfield2=val2\nMimeType=x-scheme-handler/app;application/cdf".to_string();
        let mut de = DesktopEntry::try_from(content).unwrap();
        de.delete_scheme_handler();
        assert!(!de
            .data
            .get("MimeType")
            .unwrap()
            .contains("x-scheme-handler/app"));
    }
}
