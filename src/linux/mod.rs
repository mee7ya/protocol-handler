use std::{
    collections::HashMap,
    env::var,
    error::Error,
    fs::{File, OpenOptions},
    io::Read,
    os::unix::fs::FileExt,
    str::Lines,
    usize,
};

struct DesktopEntry {
    data: HashMap<String, String>,
}

impl TryFrom<String> for DesktopEntry {
    type Error = &'static str;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let mut lines: Lines = s.lines();
        match lines.next() {
            Some(val) => {
                if val != "[Desktop Entry]" {
                    return Err("Not a desktop entry");
                }
            }
            None => {}
        }

        let mut data: HashMap<String, String> = HashMap::new();
        for line in lines {
            let split: Vec<&str> = line.split('=').collect();
            if split.len() != 2 {
                return Err("Invalid field format");
            }
            data.insert(split[0].to_string(), split[1].to_string());
        }

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
                split[scheme_handler_pos] = &entry;
                *val = split.join(";");
            }
            None => {
                self.data.insert("MimeType".to_string(), entry);
            }
        }
    }
}

fn get_file(name: &String) -> Result<File, Box<dyn Error>> {
    let home: String = var("HOME")?;
    let path: String = format!("{home}/.local/share/applications/{}.desktop", name);

    Ok(OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?)
}

pub fn register(name: &String, protocol_name: &String) -> Result<(), Box<dyn Error>> {
    let mut file = get_file(name)?;
    let mut content = String::new();

    file.read_to_string(&mut content)?;
    let mut de: DesktopEntry = DesktopEntry::try_from(content)?;

    de.insert_scheme_handler(format!("x-scheme-handler/{protocol_name}"));

    file.set_len(0)?;
    file.write_at(de.to_string().as_bytes(), 0)?;
    Ok(())
}
