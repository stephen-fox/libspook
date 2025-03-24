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
    let conf_path = match has_config() {
        Ok(opt_path) => match opt_path {
            Some(path) => path,
            None => return,
        },
        Err(e) => {
            error_message_box(format!("failed to get config path - {e}"));
            return;
        }
    };

    let conf = match Config::from_path(conf_path) {
        Ok(c) => c,
        Err(e) => {
            error_message_box(format!("failed to load config file - {e}"));
            return;
        }
    };

    for lib_path in conf.load_libraries {
        let path_str = lib_path.display().to_string();
        let mut path_str_utf16 = path_str.encode_utf16().collect::<Vec<_>>();
        path_str_utf16.push(0);

        let h_dll = unsafe { LoadLibraryW(path_str_utf16.as_ptr()) };
        if h_dll.is_null() {
            error_message_box(format!(
                "failed to load DLL ({path_str}) - last os error: {err}",
                err = std::io::Error::last_os_error()
            ));

            return;
        }
    }
}

fn has_config() -> Result<Option<PathBuf>, Box<dyn Error>> {
    let exe_path = match env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            return Err(format!("failed to get current exe path: {e}"))?;
        }
    };

    let Some(exe_name) = exe_path.file_name() else {
        return Err("failed to get exe name")?;
    };

    let Some(exe_name) = exe_name.to_str() else {
        return Err("failed to convert exe name os str to str")?;
    };

    #[allow(deprecated)]
    let Some(home_path) = home_dir() else {
        return Err("failed to get home directory")?;
    };

    let mut config_path = PathBuf::from(home_path);

    config_path.push(CONF_DIR);

    if !config_path.exists() {
        return Err("configuration directory does not exist")?;
    }

    config_path.push(String::from(exe_name) + ".conf");

    if !config_path.exists() {
        return Ok(None);
    }

    Ok(Some(config_path))
}

struct Config {
    load_libraries: Vec<PathBuf>,
}

impl Config {
    fn from_path(config_path: PathBuf) -> Result<Self, Box<dyn Error>> {
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
            load_libraries: Vec::<PathBuf>::new(),
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
                return Err(format!("line {line_num}: missing parameter"))?;
            };

            key = key.trim();

            let Some(mut value) = splitter.next() else {
                return Err(format!("line {line_num}: missing value"))?;
            };

            value = value.trim();

            match key {
                "load" => conf.load_libraries.push(PathBuf::from(value)),
                _ => {}
            }
        }

        Ok(conf)
    }
}

// https://github.com/microsoft/windows-rs/issues/973#issuecomment-1363481060
fn error_message_box(msg: String) {
    let msg = format!("🤕 {msg}\0");
    let msg = msg.encode_utf16().collect::<Vec<_>>();

    let mut title = LIBNAME.encode_utf16().collect::<Vec<_>>();
    title.push(0x00);

    unsafe {
        MessageBoxW(0, msg.as_ptr(), title.as_ptr(), 0);
    };
}
