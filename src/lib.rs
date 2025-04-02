#![allow(non_snake_case)]

#[allow(deprecated)]
use std::{
    env::{self, home_dir},
    error::Error,
    fs::File,
    io::{self, BufRead},
    path::PathBuf,
};

const NEWLINE: &str = "\r\n";

// pub type HMODULE = *mut core::ffi::c_void
// pub type HANDLE = *mut core::ffi::c_void
// pub type PCWSTR/LPWSTR = *const u16

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryW(lp_lib_file_name: *const u16) -> *mut u8;
}

#[link(name = "user32")]
extern "system" {
    fn MessageBoxW(hwnd: isize, lptext: *const u16, lpcaption: *const u16, utype: u32) -> i32;
}

#[no_mangle]
extern "system" fn DllMain(_: isize, call_reason: u32, _: *mut ()) -> bool {
    // https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain
    const DLL_PROCESS_ATTACH: u32 = 1;

    if call_reason == DLL_PROCESS_ATTACH {
        attach();
    }

    // Unloads the dll
    false
}

fn attach() {
    #[cfg(feature = "debug")]
    dbg_msg_box(format!(
        "loaded into: '{}'",
        env::args().collect::<Vec<_>>().join(" ")
    ));

    let proc_info = match ProcInfo::get() {
        Ok(info) => info,
        Err(err) => {
            err_msg_box(format!("failed to get process info - {err}"));

            return;
        }
    };

    let config = match ConfigParser::parse_default_path() {
        Ok(opt) => {
            if opt.is_none() {
                #[cfg(feature = "debug")]
                dbg_msg_box("config directory or file does not exist".into())
            }

            opt.unwrap()
        }
        Err(err) => {
            err_msg_box(format!("failed to get configuration file - {err}"));

            return;
        }
    };

    let debug = config.debug || cfg!(feature = "debug");

    if debug {
        dbg_msg_box(format!("parsed config:{NEWLINE}{NEWLINE}{config}"));
    }

    let proc_config = match config.proc_config_for_exe(&proc_info.exe_name) {
        Some(pc) => pc,
        None => {
            if debug {
                dbg_msg_box(format!("no config defined for exe {}", &proc_info.exe_name));
            }

            return;
        }
    };

    for library in &proc_config.load_libraries {
        let path_str = library.path.display().to_string();

        let mut path_str_utf16 = path_str.encode_utf16().collect::<Vec<_>>();
        path_str_utf16.push(0);

        let h_dll = unsafe { LoadLibraryW(path_str_utf16.as_ptr()) };
        if h_dll.is_null() {
            let err = std::io::Error::last_os_error();
            let code = err.raw_os_error().unwrap_or(0);

            if library.allow_init_failure && code == 1114 {
                // 1114 == DllMain returned false / init failure
                continue;
            }

            err_msg_box(format!("failed to load '{path_str}' - {err}"));

            return;
        }
    }
}

struct ProcInfo {
    exe_name: String,
}

impl ProcInfo {
    fn get() -> Result<Self, Box<dyn Error>> {
        let exe_path = match env::current_exe() {
            Ok(path) => path,
            Err(err) => {
                return Err(format!("failed to get current exe path - {err}"))?;
            }
        };

        let Some(exe_name) = exe_path.file_name() else {
            return Err("failed to get exe basename")?;
        };

        let Some(exe_name) = exe_name.to_str() else {
            return Err("failed to convert exe basename os str to str")?;
        };

        Ok(Self {
            exe_name: String::from(exe_name),
        })
    }
}

struct Config {
    debug: bool,
    proc_configs: Vec<ProcConfig>,
}

impl Config {
    fn proc_config_for_exe(self, exe_name: &str) -> Option<ProcConfig> {
        self.proc_configs
            .iter()
            .find(|config| config.exe_name == exe_name)
            .cloned()
    }
}

struct ConfigParser {
    config: Config,
    on_general: bool,
}

impl ConfigParser {
    fn parse_default_path() -> Result<Option<Config>, Box<dyn Error>> {
        match Self::default_config_path() {
            Ok(maybe_path_exists) => match maybe_path_exists {
                Some(path) => Ok(Some(Self::parse_path(&path)?)),
                None => Ok(None),
            },
            Err(err) => Err(err)?,
        }
    }

    fn default_config_path() -> Result<Option<PathBuf>, Box<dyn Error>> {
        #[allow(deprecated)]
        let Some(mut config_path) = home_dir() else {
            return Err("failed to get home directory")?;
        };

        config_path.push(String::from(".") + env!("CARGO_PKG_NAME"));

        if !config_path.exists() {
            return Ok(None);
        }

        config_path.push(String::from(env!("CARGO_PKG_NAME")) + ".conf");

        if !config_path.exists() {
            return Ok(None);
        }

        Ok(Some(config_path))
    }

    fn parse_path(config_path: &PathBuf) -> Result<Config, Box<dyn Error>> {
        let file = match File::open(config_path) {
            Ok(f) => f,
            Err(err) => Err(format!(
                "failed to open config file at '{}' - {}",
                config_path.display(),
                err
            ))?,
        };

        let mut parser = ConfigParser {
            config: Config {
                debug: false,
                proc_configs: Vec::new(),
            },
            on_general: false,
        };

        let mut buf_reader = io::BufReader::new(file);

        parser.parse(&mut buf_reader)?;

        Ok(parser.config)
    }

    fn parse<R: io::BufRead>(&mut self, buf_reader: &mut R) -> Result<(), Box<dyn Error>> {
        let mut line_num = 0;

        for line in buf_reader.lines() {
            line_num += 1;

            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    return Err(format!("line {line_num}: failed to read from config - {e}"))?
                }
            };

            match self.parse_line(line) {
                Ok(()) => {}
                Err(err) => return Err(format!("line {line_num}: {err}"))?,
            }
        }

        Ok(())
    }

    fn parse_line(&mut self, line: String) -> Result<(), Box<dyn Error>> {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            return Ok(());
        }

        let section = match ConfigParser::is_section(line) {
            Ok(s) => s,
            Err(err) => return Err(format!("failed to parse section header - {err}"))?,
        };

        if let Some(section_name) = section {
            if section_name == "general" {
                self.on_general = true;

                return Ok(());
            }

            self.on_general = false;

            self.config.proc_configs.push(ProcConfig {
                exe_name: String::from(section_name),
                load_libraries: Vec::new(),
            });

            return Ok(());
        }

        let mut splitter = line.splitn(2, '=');

        let Some(mut key) = splitter.next() else {
            return Err("missing parameter name")?;
        };

        key = key.trim();

        let Some(mut value) = splitter.next() else {
            return Err("missing value")?;
        };

        value = value.trim();

        if self.on_general {
            self.parse_general_param(key, value)
                .map_err(|err| format!("failed to parse general section - {err}"))?;
        } else {
            self.parse_proc_param(key, value)
                .map_err(|err| format!("failed to parse process section - {err}"))?;
        }

        Ok(())
    }

    fn is_section(line: &str) -> Result<Option<&str>, Box<dyn Error>> {
        if !line.starts_with('[') {
            return Ok(None);
        }

        let line = match line.strip_prefix('[') {
            Some(l) => l,
            None => return Err("missing section name and closing bracket")?,
        };

        let line = match line.strip_suffix(']') {
            Some(l) => l,
            None => return Err("missing section name")?,
        };

        let line = line.trim();

        if line.is_empty() {
            return Err("section name is empty space")?;
        }

        Ok(Some(line))
    }

    fn parse_proc_param(&mut self, key: &str, value: &str) -> Result<(), Box<dyn Error>> {
        let Some(proc_config) = self.config.proc_configs.last_mut() else {
            return Err("parameter '{key}' must be defined in a process section")?;
        };

        match key {
            "load" => proc_config.load_libraries.push(LoadConfig {
                path: PathBuf::from(value),
                allow_init_failure: false,
            }),
            "allow_init_failure" => {
                let load = match proc_config.load_libraries.last_mut() {
                    Some(lib) => lib,
                    None => {
                        return Err(format!("'{key}' must appear after 'load'"))?;
                    }
                };

                load.allow_init_failure = value
                    .parse()
                    .map_err(|err| format!("failed to parse '{key}' value - {err}"))?;
            }
            _ => return Err(format!("unknown parameter: '{key}'"))?,
        }

        Ok(())
    }

    fn parse_general_param(&mut self, key: &str, value: &str) -> Result<(), Box<dyn Error>> {
        match key {
            "debug" => {
                self.config.debug = value
                    .parse()
                    .map_err(|err| format!("failed to parse '{key}' value - {err}"))?;
            }
            _ => return Err(format!("unknown parameter: '{key}'"))?,
        };

        Ok(())
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[general]{NEWLINE}")?;

        write!(f, "debug = {}{}", self.debug, NEWLINE)?;

        for proc_config in self.proc_configs.iter().enumerate() {
            write!(f, "{}{}", NEWLINE, proc_config.1)?;
        }

        Ok(())
    }
}

#[derive(Clone)]
struct ProcConfig {
    exe_name: String,
    load_libraries: Vec<LoadConfig>,
}

impl std::fmt::Display for ProcConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]{}", self.exe_name, NEWLINE)?;

        for lib in self.load_libraries.iter().enumerate() {
            write!(f, "{}{}", lib.1, NEWLINE)?;
        }

        Ok(())
    }
}

#[derive(Clone)]
struct LoadConfig {
    path: PathBuf,
    allow_init_failure: bool,
}

impl std::fmt::Display for LoadConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "path = '{}'{NEWLINE}",
            &self.path.to_str().unwrap_or("???"),
        )?;

        write!(f, "allow_init_failure = {}", self.allow_init_failure)
    }
}

fn err_msg_box(msg: String) {
    msg_box(format!("🤕 {msg}"));
}

fn dbg_msg_box(msg: String) {
    msg_box(format!("debug: {msg}"))
}

fn msg_box(msg: String) {
    // https://github.com/microsoft/windows-rs/issues/973#issuecomment-1363481060
    let mut msg = msg.encode_utf16().collect::<Vec<_>>();
    msg.push(0x00);

    let mut title = format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        .encode_utf16()
        .collect::<Vec<_>>();

    title.push(0x00);

    unsafe {
        MessageBoxW(0, msg.as_ptr(), title.as_ptr(), 0);
    };
}
