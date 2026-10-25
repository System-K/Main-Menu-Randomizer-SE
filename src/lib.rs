use bitflags::bitflags;
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ffi::c_void,
    fmt::Display,
    fs::File,
    io::{Read, Write},
    os::raw::c_char,
    path::{Path, PathBuf, StripPrefixError},
};
use thiserror::Error;

#[derive(Debug, Deserialize, PartialEq, PartialOrd)]
enum LogLevel {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => "ERROR".fmt(f),
            LogLevel::Warn => "WARN".fmt(f),
            LogLevel::Info => "INFO".fmt(f),
            LogLevel::Debug => "DEBUG".fmt(f),
            LogLevel::Trace => "TRACE".fmt(f),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Config {
    source: Vec<Directory>,
    destination: PathBuf,
    filter: Filter,
    log_level: LogLevel,
    #[serde(default)]
    anniversary_workaroud: bool,
    #[serde(default)]
    backup: Option<Backup>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Backup {
    directory: PathBuf,
    #[serde(default)]
    filter: BackupFilter,
}

#[derive(Debug, Deserialize, Default)]
enum BackupFilter {
    Hash,
    New,
    #[default]
    None,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Directory {
    directory: PathBuf,
    filter: Option<Filter>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(try_from = "String")]
struct Filter(Regex);

impl std::fmt::Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}

impl TryFrom<String> for Filter {
    type Error = regex::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let regex = Regex::new(&value)?;

        Ok(Self(regex))
    }
}

#[cfg(not(feature = "ae"))]
#[derive(Debug)]
#[repr(C)]
pub struct SKSEPluginVersionData {
    data_version: u32,
    plugin_version: u32,
    name: [u8; 256],
    author: [u8; 256],
    support_email: [u8; 252],
    version_independence: VersionIndebendence,
    compatible_versions: [u32; 16],
    se_version_required: u32,
}

#[cfg(feature = "ae")]
#[derive(Debug)]
#[repr(C)]
pub struct SKSEPluginVersionData {
    data_version: u32,
    plugin_version: u32,
    name: [u8; 256],
    author: [u8; 256],
    support_email: [u8; 252],
    version_independence_ex: VersionIndebendenceEx,
    version_independence: VersionIndebendence,
    compatible_versions: [u32; 16],
    se_version_required: u32,
}

const fn zero_pad_u8<const N: usize, const M: usize>(arr: &[u8; N]) -> [u8; M] {
    let mut m = [0; M];
    let mut i = 0;
    while i < N {
        m[i] = arr[i];
        i += 1;
    }
    m
}

static NAME: &[u8; 21] = b"MainMenuRandomizerSE\0";
static AUTHOR: &[u8; 10] = b"Lanastara\0";
static EMAIL: &[u8; 24] = b"lilly.mannhal@gmail.com\0";

bitflags! {
    #[derive(Debug)]
    #[repr(C)]
    struct VersionIndebendence: u32{
        const ADDRESS_LIBRARY_POST_AE = 1<<0;
        const SIGNATURES = 1 << 1;
        const STRUCTS_POST629 = 1 << 2;
    }
}

#[cfg(feature = "ae")]
bitflags! {
    #[derive(Debug)]
    #[repr(C)]
    struct VersionIndebendenceEx: u32{
        const NO_STRUCT_USE = 1<<0;
    }
}

#[no_mangle]
pub static SKSEPlugin_Version: SKSEPluginVersionData = SKSEPluginVersionData {
    data_version: 1,
    plugin_version: 4,
    name: zero_pad_u8(NAME),
    author: zero_pad_u8(AUTHOR),
    support_email: zero_pad_u8(EMAIL),
    #[cfg(feature = "ae")]
    version_independence_ex: VersionIndebendenceEx::NO_STRUCT_USE,
    version_independence: VersionIndebendence::ADDRESS_LIBRARY_POST_AE,
    compatible_versions: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    se_version_required: 0,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("IOError: {0:?}")]
    IoError(#[from] std::io::Error),
    #[error("Replacer \"{0}\" does not contain a Data Directory")]
    DataMissing(PathBuf),
    #[error("SourceDirectory \"{0}\" does not exist")]
    SourceMissing(PathBuf),
    #[error("SourceDirectory \"{0}\" is empty")]
    SourceEmpty(PathBuf),
    #[error("Could not read Config File")]
    ConfigError,
    #[error("Could not read Hashes File")]
    HashesError,

    #[error("Could not evaluate relative path")]
    StripPrefixError(#[from] StripPrefixError),
}

struct Logger {
    log_level: LogLevel,
    file: std::fs::File,
}

impl Logger {
    fn new(file: File) -> Self {
        Self {
            file,
            log_level: LogLevel::Trace,
        }
    }

    fn set_level(&mut self, level: LogLevel) {
        self.log_level = level;
    }

    fn log<TMessage: AsRef<str>>(
        &mut self,
        level: LogLevel,
        message: TMessage,
    ) -> Result<(), std::io::Error> {
        if level <= self.log_level {
            writeln!(
                &self.file,
                "{} {} {}",
                chrono::Local::now(),
                level,
                message.as_ref()
            )?;

            self.file.flush()?
        }

        Ok(())
    }

    fn error<TMessage: AsRef<str>>(&mut self, message: TMessage) -> Result<(), std::io::Error> {
        self.log(LogLevel::Error, message)
    }

    fn _warn<TMessage: AsRef<str>>(&mut self, message: TMessage) -> Result<(), std::io::Error> {
        self.log(LogLevel::Warn, message)
    }

    fn info<TMessage: AsRef<str>>(&mut self, message: TMessage) -> Result<(), std::io::Error> {
        self.log(LogLevel::Info, message)
    }

    fn debug<TMessage: AsRef<str>>(&mut self, message: TMessage) -> Result<(), std::io::Error> {
        self.log(LogLevel::Debug, message)
    }

    fn trace<TMessage: AsRef<str>>(&mut self, message: TMessage) -> Result<(), std::io::Error> {
        self.log(LogLevel::Trace, message)
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        let _ = self.file.flush();
    }
}

fn handle_dir(
    directory: Directory,
    filter: Filter,
    logger: &mut Logger,
    destination: &PathBuf,
    ae_workaround: bool,
    backup: &Option<Backup>,
    backup_cache: &mut BackupCache,
) -> Result<(), Error> {
    let filter = directory.filter.unwrap_or(filter);
    logger.debug(format!(
        "Using Source directory: '{:?}' with filter '{}'",
        directory.directory, filter
    ))?;
    if !directory.directory.exists() {
        return Err(Error::SourceMissing(directory.directory));
    }

    let read_dir = directory.directory.read_dir()?;
    let directories: Vec<_> = read_dir
        .filter_map(|r| r.ok())
        .filter(|dir| dir.metadata().map(|m| m.is_dir()).unwrap_or_default())
        .collect();

    logger.debug(format!(
        "Found the following possible replacers: {:?}",
        directories
    ))?;

    if directories.is_empty() {
        return Err(Error::SourceEmpty(directory.directory));
    }

    let index = rand::thread_rng().gen_range(0..directories.len());
    let dir = &directories[index].path();

    logger.info(format!("Using Replacer at: {:?}", dir))?;

    let data = dir.join("data");

    if !data.exists() {
        return Err(Error::DataMissing(directory.directory));
    }

    logger.debug(format!("Copying files to: '{:?}'", destination))?;

    let walkdir = walkdir::WalkDir::new(&data);

    let mut logo_path = Path::new("meshes").to_path_buf();
    logo_path.push("interface");
    logo_path.push("logo");

    let mut logo2_path = logo_path.clone();
    logo2_path.push("logo01ae.nif");
    logo_path.push("logo.nif");

    for d in walkdir {
        if let Ok(d) = d {
            let abs_src = d.path();
            if abs_src.is_file() {
                let rel_src = abs_src.strip_prefix(&data)?;

                if let Some(rel_src_str) = rel_src.to_str() {
                    if filter.0.is_match(rel_src_str) {
                        let abs_dst = destination.join(rel_src);

                        let dst_dir = abs_dst.parent();

                        if let Some(dst_dir) = dst_dir {
                            if !dst_dir.exists() {
                                logger
                                    .debug(format!("Creating missing Directory: '{:?}", dst_dir))?;
                                std::fs::create_dir_all(dst_dir)?;
                            }
                        }

                        logger.trace(format!("Copying file: '{:?}'", rel_src))?;

                        if let Some(backup) = backup {
                            if abs_dst.exists() {
                                let mut hash = None;
                                let needs_backup = match backup.filter {
                                    BackupFilter::Hash => {
                                        let tmp = backup_cache
                                            .cache
                                            .get(rel_src)
                                            .map_or(true, |e| check_file(e, &abs_dst));
                                        hash = Some(calc_hash(abs_src));
                                        tmp
                                    }
                                    BackupFilter::New => !backup_cache.cache.contains_key(rel_src),
                                    BackupFilter::None => true,
                                };

                                if needs_backup {
                                    let backup_dst =
                                        backup.directory.join(&backup_cache.subdir).join(rel_src);
                                    let backup_dir = backup_dst.parent();
                                    if let Some(backup_dir) = backup_dir {
                                        if !backup_dir.exists() {
                                            logger.debug(format!(
                                                "Creating missing backup Directory: '{:?}",
                                                backup_dir
                                            ))?;
                                            std::fs::create_dir_all(backup_dir)?;
                                        }
                                    }

                                    logger.trace(format!(
                                        "Creating backup for file: '{:?}'",
                                        rel_src
                                    ))?;
                                    std::fs::copy(&abs_dst, backup_dst)?;
                                }

                                backup_cache.cache.insert(rel_src.to_path_buf(), hash);
                            }
                        }

                        std::fs::copy(abs_src, abs_dst)?;

                        if ae_workaround && rel_src == logo_path {
                            let abs_dst = destination.join(&logo2_path);
                            logger.trace(format!("Copying file to '{:?}'", abs_dst))?;
                            std::fs::copy(abs_src, abs_dst)?;
                        }
                    } else {
                        logger.info(format!("File '{:?}' skipped due to filter.", rel_src))?;
                    }
                }
            }
        } else {
            logger.error("Invalid Directory Entry encountered")?;
        }
    }

    Ok(())
}

const BUFFER_SIZE: usize = 102400;

fn check_file(entry: &Option<HashEntry>, path: &Path) -> bool {
    let Some(entry) = entry else {
        return true;
    };

    if entry.size != file_size(path) {
        return true;
    }
    file_hash::<BUFFER_SIZE>(path) != entry.hash
}

fn calc_hash(path: &Path) -> HashEntry {
    let size = file_size(path);

    let hash = file_hash::<BUFFER_SIZE>(path);

    HashEntry { size, hash }
}

pub fn file_hash<const N: usize>(path: &Path) -> u32 {
    let mut buff = [0u8; N];

    inner_file_hash(path, &mut buff)
}
pub fn inner_file_hash(path: &Path, buffer: &mut [u8]) -> u32 {
    let Ok(mut file) = std::fs::File::open(path) else {
        return 0;
    };
    let mut crc = crc32fast::Hasher::new();

    while let Ok(i) = file.read(buffer) {
        if i == 0 {
            break;
        }

        crc.update(&buffer[..i]);
    }
    crc.finalize()
}

fn file_size(path: &Path) -> u64 {
    path.metadata().map_or(0u64, |m| m.len())
}

pub fn replace() -> Result<(), Error> {
    let start = std::time::Instant::now();
    let mut log_file = match std::fs::File::create(".\\data\\skse\\plugins\\MainMenuRandomizer.log")
    {
        Ok(v) => v,
        Err(e) => return Err(e.into()),
    };
    if let Err(e) = log_file.write_fmt(format_args!(
        "{} MainMenuRandomizerSE: Starting Version {}\n",
        chrono::Local::now(),
        std::env!("CARGO_PKG_VERSION")
    )) {
        return Err(e.into());
    };
    if let Err(e) = log_file.flush() {
        return Err(e.into());
    };
    let mut logger = Logger::new(log_file);
    match replace_inner(&mut logger, start) {
        Ok(o) => Ok(o),
        Err(e) => {
            let _ = logger.error(format!("{}", e));
            Err(e)
        }
    }
}

fn replace_inner(logger: &mut Logger, start: std::time::Instant) -> Result<(), Error> {
    let mut config_file_buffer = Vec::new();
    std::fs::File::open(".\\data\\skse\\plugins\\MainMenuRandomizer.config.toml")?
        .read_to_end(&mut config_file_buffer)?;

    let config: Config = match toml::from_slice(&config_file_buffer) {
        Ok(c) => c,
        Err(e) => {
            let _ = logger.error(format!("Error in Configfile: '{:?}'\n", e));
            return Err(Error::ConfigError);
        }
    };
    logger.set_level(config.log_level);

    let ae_workaroud = config.anniversary_workaroud;

    logger.trace("Config Loaded")?;

    if !config.destination.exists() {
        logger.error("Destination Directory does not exist")?;
    }

    let mut cache = BackupCache::new(&config.backup, logger);
    for directory in config.source {
        if let Err(e) = handle_dir(
            directory,
            config.filter.clone(),
            logger,
            &config.destination,
            ae_workaroud,
            &config.backup,
            &mut cache,
        ) {
            logger.error(format!("{}", e))?;
        }
    }

    cache.save(&config.backup, logger);

    logger.info(format!(
        "Replacing took: {}ms",
        std::time::Instant::elapsed(&start).as_millis()
    ))?;

    Ok(())
}

struct BackupCache {
    subdir: String,
    cache: HashMap<PathBuf, Option<HashEntry>>,
}
impl BackupCache {
    fn new(backup: &Option<Backup>, logger: &mut Logger) -> Self {
        let timestamp = format!("{}", chrono::Local::now().format("%d%m%Y%_H%M%S"));
        match backup {
            Some(b) => {
                let hashes = hashes_name(b);
                let Ok(file) = std::fs::File::open(hashes) else {
                    let _ = logger.info("Hashes File does not exist");

                    return Self {
                        subdir: timestamp,
                        cache: HashMap::new(),
                    };
                };

                let Ok(cache) = serde_json::from_reader(&file) else {
                    let _ = logger.error("Failed to parse Hashes File");
                    return Self {
                        subdir: timestamp,
                        cache: HashMap::new(),
                    };
                };

                Self {
                    subdir: timestamp,
                    cache,
                }
            }
            None => Self {
                subdir: timestamp,
                cache: HashMap::new(),
            },
        }
    }

    fn save(self, backup: &Option<Backup>, logger: &mut Logger) {
        let Some(backup) = backup else {
            return;
        };

        let _ = logger.info("writing hashes file");
        if !backup.directory.exists() {
            if std::fs::create_dir_all(&backup.directory).is_ok() {
                let _ = logger.error("Failed to create backup dir");
                return;
            }
        }

        let hashes = hashes_name(backup);

        let Ok(file) = std::fs::File::create(hashes) else {
            let _ = logger.info("Failed to create hashes file");

            return;
        };
        let Ok(()) = serde_json::to_writer_pretty(file, &self.cache) else {
            let _ = logger.error("Failed to serialize hashes");
            return;
        };
    }
}

fn hashes_name(b: &Backup) -> PathBuf {
    let hashes = b.directory.join("hashes");
    hashes
}

#[derive(Debug, Deserialize, Serialize)]
struct HashEntry {
    size: u64,
    hash: u32,
}

#[repr(C)]
pub struct PluginInfo {
    info_version: u32,
    name: *const c_char,
    version: u32,
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn SKSEPlugin_Query(_skse: *const c_void, info: *mut PluginInfo) -> bool {
    let info = unsafe { &mut *info };

    info.info_version = 1;
    info.name = NAME.as_ptr() as *const c_char;
    info.version = 1;
    true
}

#[no_mangle]
pub extern "C" fn SKSEPlugin_Load(_skse: *const c_void) -> bool {
    let _ = replace();
    true
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn F4SEPlugin_Query(_skse: *const c_void, info: *mut PluginInfo) -> bool {
    let info = unsafe { &mut *info };

    info.info_version = 1;
    info.name = NAME.as_ptr() as *const c_char;
    info.version = 1;
    true
}

#[no_mangle]
pub extern "C" fn F4SEPlugin_Load(_skse: *const c_void) -> bool {
    let _ = replace();
    true
}
