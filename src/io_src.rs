use std::{
    fmt,
    io::{Read, Write},
    path::PathBuf,
    str::FromStr,
};

#[derive(Debug, Clone)]
pub enum InputSource {
    File(PathBuf),
    Content(String),
    Stdin,
}
impl InputSource {
    pub fn resolve(self) -> anyhow::Result<String> {
        match self {
            InputSource::File(path) => Ok(std::fs::read_to_string(path)?),
            InputSource::Content(content) => Ok(content),
            InputSource::Stdin => {
                let mut buffer = String::new();
                std::io::stdin().read_to_string(&mut buffer)?;
                Ok(buffer)
            }
        }
    }
}

impl FromStr for InputSource {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "-" {
            Ok(InputSource::Stdin)
        } else if PathBuf::from(s).exists() {
            Ok(InputSource::File(PathBuf::from(s)))
        } else {
            Ok(InputSource::Content(s.to_string()))
        }
    }
}

impl fmt::Display for InputSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputSource::File(file) => write!(f, "Input from file path: {}", file.display()),
            InputSource::Content(content) => write!(f, "Content Input: {content}"),
            InputSource::Stdin => write!(f, "Input from stdin"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum OutputSource {
    File(PathBuf),
    Stdout,
}

impl fmt::Display for OutputSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputSource::File(file) => write!(f, "Output to file path: {}", file.display()),
            OutputSource::Stdout => write!(f, "Output to stdout"),
        }
    }
}

impl FromStr for OutputSource {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "-" {
            Ok(OutputSource::Stdout)
        } else {
            Ok(OutputSource::File(PathBuf::from(s)))
        }
    }
}

impl OutputSource {
    pub fn resolve(&self) -> anyhow::Result<()> {
        match self {
            OutputSource::File(file) => {
                if let Some(parent) = file.parent() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            OutputSource::Stdout => {}
        }
        Ok(())
    }
    pub fn write(&self, content: &str) -> anyhow::Result<()> {
        match self {
            OutputSource::File(file) => {
                std::fs::write(file, content)?;
            }
            OutputSource::Stdout => {
                println!("{content}");
            }
        }
        Ok(())
    }
}
