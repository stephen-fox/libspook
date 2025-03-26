#![allow(non_snake_case)]

#[allow(deprecated)]
use std::{
    env::{self, home_dir},
    error::Error,
    fs::File,
    io::{self, BufRead},
    path::PathBuf,
};

const LIBNAME: &str = "libspook";
const CONF_DIR: &str = ".libspook";
const NEWLINE: &str = "\r\n";

// pub type HMODULE = *mut core::ffi::c_void
// pub type HANDLE = *mut core::ffi::c_void
// pub type PCWSTR = *const u16 (and LPWSTR)

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryW(lp_lib_file_name: *const u16) -> *mut u8;
}

#[link(name = "user32")]
extern "system" {
    fn MessageBoxW(hwnd: isize, lptext: *const u16, lpcaption: *const u16, utype: u32) -> i32;
}

#[no_mangle]
#[allow(unused_variables)]
extern "system" fn DllMain(_: isize, call_reason: u32, _: *mut ()) -> bool {
    // https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain
    const DLL_PROCESS_ATTACH: u32 = 1u32;

    if call_reason == DLL_PROCESS_ATTACH {
        attach();
    }

    // Unloads the dll
    false
}

fn attach() {
    let proc_info = match ProcInfo::get() {
        Ok(info) => info,
        Err(err) => {
            err_msg_box(format!("failed to get process info - {err}"));
            return;
        }
    };

    #[cfg(feature = "debug")]
    dbg_msg_box(format!(
        "loaded into: '{}'",
        env::args().collect::<Vec<_>>().join(" ")
    ));

    let conf_path = match has_config(&proc_info.exe_name) {
        Ok(opt_path) => match opt_path {
            Some(path) => path,
            None => {
                #[cfg(feature = "debug")]
                dbg_msg_box("no config file or directory available".into());

                return;
            }
        },
        Err(e) => {
            err_msg_box(format!("failed to get config path - {e}"));
            return;
        }
    };

    let conf = match Config::from_path(&conf_path) {
        Ok(c) => c,
        Err(e) => {
            err_msg_box(format!("failed to parse config file - {e}"));
            return;
        }
    };

    #[cfg(feature = "debug")]
    dbg_msg_box(format!(
        "config file: '{}'{}{}config contents:{}{}{}",
        conf_path.clone().display(),
        NEWLINE,
        NEWLINE,
        NEWLINE,
        NEWLINE,
        conf
    ));

    for library in conf.load_libraries {
        let path_str = library.path.display().to_string();
        let mut path_str_utf16 = path_str.encode_utf16().collect::<Vec<_>>();
        path_str_utf16.push(0);

        let h_dll = unsafe { LoadLibraryW(path_str_utf16.as_ptr()) };
        if h_dll.is_null() {
            err_msg_box(format!(
                "failed to load DLL ({path_str}) - last os error: {err}",
                err = std::io::Error::last_os_error()
            ));

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

fn has_config(exe_name: &str) -> Result<Option<PathBuf>, Box<dyn Error>> {
    #[allow(deprecated)]
    let Some(home_path) = home_dir() else {
        return Err("failed to get home directory")?;
    };

    let mut config_path = PathBuf::from(home_path);

    config_path.push(CONF_DIR);

    if !config_path.exists() {
        return Ok(None);
    }

    config_path.push(String::from(exe_name) + ".conf");

    if !config_path.exists() {
        return Ok(None);
    }

    Ok(Some(config_path))
}

struct Config {
    load_libraries: Vec<LoadConfig>,
}

impl Config {
    fn from_path(config_path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let f = match File::open(&config_path) {
            Ok(f) => f,
            Err(e) => Err(format!(
                "failed to open config file at '{}' - {}",
                config_path.display(),
                e
            ))?,
        };

        let mut line_num: u32 = 0;

        let mut conf = Self {
            load_libraries: Vec::new(),
        };

        for line in io::BufReader::new(f).lines() {
            line_num += 1;

            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    return Err(format!("line {line_num}: failed to read from config - {e}"))?
                }
            };

            let mut splitter = line.splitn(2, '=');

            let Some(mut key) = splitter.next() else {
                return Err(format!("line {line_num}: missing parameter name"))?;
            };

            key = key.trim();

            if key.starts_with('#') {
                continue;
            }

            let Some(mut value) = splitter.next() else {
                return Err(format!("line {line_num}: missing value"))?;
            };

            value = value.trim();

            match key {
                "load" => conf.load_libraries.push(LoadConfig {
                    path: PathBuf::from(value),
                }),
                _ => {}
            }
        }

        Ok(conf)
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "load_libraries:{}", NEWLINE)?;

        for lib in self.load_libraries.iter().enumerate() {
            write!(f, "  - '{}'{}", lib.1, NEWLINE)?;
        }

        Ok(())
    }
}

struct LoadConfig {
    path: PathBuf,
}

impl std::fmt::Display for LoadConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "path: '{}'", &self.path.to_str().unwrap_or("???"))
    }
}

// https://github.com/microsoft/windows-rs/issues/973#issuecomment-1363481060
fn err_msg_box(msg: String) {
    msg_box(format!("🤕 {msg}"));
}

#[cfg(feature = "debug")]
fn dbg_msg_box(msg: String) {
    msg_box(format!("debug: {msg}"))
}

fn msg_box(msg: String) {
    let mut msg = msg.encode_utf16().collect::<Vec<_>>();
    msg.push(0x00);

    let mut title = LIBNAME.encode_utf16().collect::<Vec<_>>();
    title.push(0x00);

    unsafe {
        MessageBoxW(0, msg.as_ptr(), title.as_ptr(), 0);
    };
}
