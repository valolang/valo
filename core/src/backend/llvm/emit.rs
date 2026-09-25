use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::mir::ir::Module;

use super::{BackendError, Target, render_module};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitKind {
    LlvmIr,
    Object,
    Executable,
}

#[derive(Debug, Clone)]
pub struct NativeOptions {
    pub output: PathBuf,
    pub kind: EmitKind,
    pub optimize: bool,
}

#[derive(Debug, Clone)]
pub struct Artifact {
    pub path: PathBuf,
    pub target: Target,
    pub llvm_version: String,
}

#[derive(Debug, Clone)]
pub struct LlvmTools {
    pub clang: PathBuf,
    pub opt: PathBuf,
    pub llc: PathBuf,
    pub version: String,
    pub target: Target,
}

impl LlvmTools {
    /// VALO_LLVM_BIN may name an extracted official LLVM bin directory. If
    /// absent, the three tools are resolved through PATH.
    pub fn discover() -> Result<Self, BackendError> {
        let bin = std::env::var_os("VALO_LLVM_BIN").map(PathBuf::from);
        let tool = |name: &str| {
            let filename = if cfg!(windows) {
                format!("{name}.exe")
            } else {
                name.into()
            };
            bin.as_ref()
                .map_or_else(|| PathBuf::from(&filename), |root| root.join(&filename))
        };
        let clang = tool("clang");
        let opt = tool("opt");
        let llc = tool("llc");
        let version_output = command_output(&clang, ["--version"], "LLVM discovery")?;
        let major = llvm_major(&version_output).ok_or_else(|| {
            BackendError::new("LLVM discovery", "cannot determine clang LLVM version")
        })?;
        if major < 18 {
            return Err(BackendError::new(
                "LLVM discovery",
                format!("LLVM {major} is too old; LLVM 18 or newer is required"),
            ));
        }
        for required in [&opt, &llc] {
            let output = command_output(required, ["--version"], "LLVM discovery")?;
            if llvm_major(&output) != Some(major) {
                return Err(BackendError::new(
                    "LLVM discovery",
                    format!("{} must use LLVM major version {major}", required.display()),
                ));
            }
        }
        let version = version_output
            .lines()
            .next()
            .unwrap_or("unknown LLVM")
            .to_string();
        let mut child = Command::new(&clang)
            .args(["-x", "c", "-S", "-emit-llvm", "-o", "-", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| BackendError::new("target", format!("{}: {e}", clang.display())))?;
        child
            .stdin
            .take()
            .expect("piped")
            .write_all(b"int valo_target_probe;")
            .map_err(|e| BackendError::new("target", e.to_string()))?;
        let output = child
            .wait_with_output()
            .map_err(|e| BackendError::new("target", e.to_string()))?;
        if !output.status.success() {
            return Err(BackendError::new(
                "target",
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        let target = Target::from_clang_ir(&String::from_utf8_lossy(&output.stdout))?;
        Ok(Self {
            clang,
            opt,
            llc,
            version,
            target,
        })
    }
}

fn llvm_major(output: &str) -> Option<u32> {
    let start = output.find("version ")? + "version ".len();
    let digits = output[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    digits.parse().ok()
}

fn command_output<I, S>(
    command: &Path,
    args: I,
    stage: &'static str,
) -> Result<String, BackendError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|e| BackendError::new(stage, format!("{}: {e}", command.display())))?;
    if !output.status.success() {
        return Err(BackendError::new(
            stage,
            format!(
                "{} exited {}: {}",
                command.display(),
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

static NEXT_WORKSPACE: AtomicU64 = AtomicU64::new(0);

struct BuildWorkspace(PathBuf);

impl BuildWorkspace {
    fn create() -> Result<Self, BackendError> {
        for _ in 0..100 {
            let id = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("valo-llvm-{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(BackendError::new("artifact", error.to_string())),
            }
        }
        Err(BackendError::new(
            "artifact",
            "could not reserve a temporary LLVM build directory",
        ))
    }
}

impl Drop for BuildWorkspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}

pub fn build(
    module: &Module,
    tools: &LlvmTools,
    options: &NativeOptions,
) -> Result<Artifact, BackendError> {
    let ir = render_module(module, &tools.target)?;
    let output = &options.output;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|e| BackendError::new("artifact", e.to_string()))?;
    let workspace = BuildWorkspace::create()?;
    let ir_path = workspace.0.join("module.ll");
    fs::write(&ir_path, ir).map_err(|e| BackendError::new("LLVM IR emission", e.to_string()))?;
    command_output(
        &tools.opt,
        [
            "-passes=verify",
            "-disable-output",
            ir_path
                .to_str()
                .ok_or_else(|| BackendError::new("artifact", "non-Unicode LLVM IR path"))?,
        ],
        "LLVM verification",
    )?;
    if options.kind == EmitKind::LlvmIr {
        fs::copy(&ir_path, output).map_err(|e| BackendError::new("artifact", e.to_string()))?;
        return Ok(Artifact {
            path: output.clone(),
            target: tools.target.clone(),
            llvm_version: tools.version.clone(),
        });
    }
    let optimized = workspace.0.join("module.opt.ll");
    if options.optimize {
        command_output(
            &tools.opt,
            [
                "-passes=default<O2>",
                "-S",
                ir_path
                    .to_str()
                    .ok_or_else(|| BackendError::new("artifact", "non-Unicode LLVM IR path"))?,
                "-o",
                optimized.to_str().ok_or_else(|| {
                    BackendError::new("artifact", "non-Unicode optimized IR path")
                })?,
            ],
            "LLVM optimization",
        )?;
        command_output(
            &tools.opt,
            [
                "-passes=verify",
                "-disable-output",
                optimized.to_str().expect("checked"),
            ],
            "LLVM verification",
        )?;
    }
    let input = if options.optimize {
        &optimized
    } else {
        &ir_path
    };
    let object = workspace.0.join(if cfg!(windows) {
        "module.obj"
    } else {
        "module.o"
    });
    command_output(
        &tools.llc,
        [
            "-filetype=obj",
            "-o",
            object
                .to_str()
                .ok_or_else(|| BackendError::new("artifact", "non-Unicode object path"))?,
            input
                .to_str()
                .ok_or_else(|| BackendError::new("artifact", "non-Unicode LLVM IR path"))?,
        ],
        "object emission",
    )?;
    if options.kind == EmitKind::Object {
        fs::copy(&object, output).map_err(|e| BackendError::new("artifact", e.to_string()))?;
    } else {
        let runtime_source = Path::new(env!("CARGO_MANIFEST_DIR")).join("native_runtime/string.c");
        let runtime_object = workspace.0.join(if cfg!(windows) {
            "valo_runtime.obj"
        } else {
            "valo_runtime.o"
        });
        command_output(
            &tools.clang,
            [
                OsStr::new("-std=c11"),
                OsStr::new("-c"),
                runtime_source.as_os_str(),
                OsStr::new("-o"),
                runtime_object.as_os_str(),
            ],
            "runtime compilation",
        )?;
        command_output(
            &tools.clang,
            [
                object.as_os_str(),
                runtime_object.as_os_str(),
                OsStr::new("-o"),
                output.as_os_str(),
            ],
            "linking",
        )?;
    }
    Ok(Artifact {
        path: output.clone(),
        target: tools.target.clone(),
        llvm_version: tools.version.clone(),
    })
}
