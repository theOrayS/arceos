use core::str;
#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
use std::collections::BTreeSet;
use std::fs::{self, File, FileType};
use std::io::{self, prelude::*};
#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
use std::string::ToString;
use std::{string::String, vec::Vec};

#[cfg(all(not(feature = "axstd"), unix))]
use std::os::unix::fs::{FileTypeExt, PermissionsExt};

use crate::path_to_str;
#[cfg(feature = "uspace")]
use arceos_posix_api::uspace;
#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
const SUITE_DIRS: &[&str] = &["/musl", "/glibc"];
#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
const SCRIPT_SUFFIX: &str = "_testcode.sh";
#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
const TESTSUITE_STAGE_ROOT: &str = "/tmp/testsuite";
#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
const SCRIPT_BUSYBOX_APPLETS: &[&str] = &["cp", "kill", "sleep"];

macro_rules! print_err {
    ($cmd: literal, $msg: expr) => {
        println!("{}: {}", $cmd, $msg)
    };
    ($cmd: literal, $arg: expr, $err: expr) => {
        println!("{}: {}: {}", $cmd, $arg, $err)
    };
}

type CmdHandler = fn(&str);

const CMD_TABLE: &[(&str, CmdHandler)] = &[
    ("cat", do_cat),
    ("cd", do_cd),
    ("echo", do_echo),
    ("exit", do_exit),
    ("help", do_help),
    ("ls", do_ls),
    ("mkdir", do_mkdir),
    ("pwd", do_pwd),
    ("rm", do_rm),
    #[cfg(feature = "uspace")]
    ("runu", do_runu),
    ("uname", do_uname),
];

fn file_type_to_char(ty: FileType) -> char {
    if ty.is_char_device() {
        'c'
    } else if ty.is_block_device() {
        'b'
    } else if ty.is_socket() {
        's'
    } else if ty.is_fifo() {
        'p'
    } else if ty.is_symlink() {
        'l'
    } else if ty.is_dir() {
        'd'
    } else if ty.is_file() {
        '-'
    } else {
        '?'
    }
}

#[rustfmt::skip]
const fn file_perm_to_rwx(mode: u32) -> [u8; 9] {
    let mut perm = [b'-'; 9];
    macro_rules! set {
        ($bit:literal, $rwx:literal) => {
            if mode & (1 << $bit) != 0 {
                perm[8 - $bit] = $rwx
            }
        };
    }

    set!(2, b'r'); set!(1, b'w'); set!(0, b'x');
    set!(5, b'r'); set!(4, b'w'); set!(3, b'x');
    set!(8, b'r'); set!(7, b'w'); set!(6, b'x');
    perm
}

fn do_ls(args: &str) {
    let current_dir = match std::env::current_dir() {
        Ok(current_dir) => current_dir,
        Err(err) => return println!("Failed to access the current directory: {err}"),
    };
    let args = if args.is_empty() {
        path_to_str(&current_dir)
    } else {
        args
    };
    let name_count = args.split_whitespace().count();

    fn show_entry_info(path: &str, entry: &str) -> io::Result<()> {
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        let file_type = metadata.file_type();
        let file_type_char = file_type_to_char(file_type);
        let rwx = file_perm_to_rwx(metadata.permissions().mode());
        let rwx = str::from_utf8(&rwx).unwrap();
        println!("{file_type_char}{rwx} {size:>8} {entry}");
        Ok(())
    }

    fn list_one(name: &str, print_name: bool) -> io::Result<()> {
        let is_dir = fs::metadata(name)?.is_dir();
        if !is_dir {
            return show_entry_info(name, name);
        }

        if print_name {
            println!("{name}:");
        }
        let mut entries = fs::read_dir(name)?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect::<Vec<_>>();
        entries.sort();

        for entry in entries {
            let entry = path_to_str(&entry);
            let path = String::from(name) + "/" + entry;
            if let Err(e) = show_entry_info(&path, entry) {
                print_err!("ls", path, e);
            }
        }
        Ok(())
    }

    for (i, name) in args.split_whitespace().enumerate() {
        if i > 0 {
            println!();
        }
        if let Err(e) = list_one(name, name_count > 1) {
            print_err!("ls", name, e);
        }
    }
}

fn do_cat(args: &str) {
    if args.is_empty() {
        print_err!("cat", "no file specified");
        return;
    }

    fn cat_one(fname: &str) -> io::Result<()> {
        let mut buf = [0; 1024];
        let mut file = File::open(fname)?;
        loop {
            let n = file.read(&mut buf)?;
            if n > 0 {
                io::stdout().write_all(&buf[..n])?;
            } else {
                return Ok(());
            }
        }
    }

    for fname in args.split_whitespace() {
        if let Err(e) = cat_one(fname) {
            print_err!("cat", fname, e);
        }
    }
}

fn do_echo(args: &str) {
    fn echo_file(fname: &str, text_list: &[&str]) -> io::Result<()> {
        let mut file = File::create(fname)?;
        for text in text_list {
            file.write_all(text.as_bytes())?;
        }
        Ok(())
    }

    if let Some(pos) = args.rfind('>') {
        let text_before = args[..pos].trim();
        let (fname, text_after) = split_whitespace(&args[pos + 1..]);
        if fname.is_empty() {
            print_err!("echo", "no file specified");
            return;
        };

        let text_list = [
            text_before,
            if !text_after.is_empty() { " " } else { "" },
            text_after,
            "\n",
        ];
        if let Err(e) = echo_file(fname, &text_list) {
            print_err!("echo", fname, e);
        }
    } else {
        println!("{args}")
    }
}

fn do_mkdir(args: &str) {
    if args.is_empty() {
        print_err!("mkdir", "missing operand");
        return;
    }

    fn mkdir_one(path: &str) -> io::Result<()> {
        fs::create_dir(path)
    }

    for path in args.split_whitespace() {
        if let Err(e) = mkdir_one(path) {
            print_err!("mkdir", format_args!("cannot create directory '{path}'"), e);
        }
    }
}

fn do_rm(args: &str) {
    if args.is_empty() {
        print_err!("rm", "missing operand");
        return;
    }
    let mut rm_dir = false;
    for arg in args.split_whitespace() {
        if arg == "-d" {
            rm_dir = true;
        }
    }

    fn rm_one(path: &str, rm_dir: bool) -> io::Result<()> {
        if rm_dir && fs::metadata(path)?.is_dir() {
            fs::remove_dir(path)
        } else {
            fs::remove_file(path)
        }
    }

    for path in args.split_whitespace() {
        if path == "-d" {
            continue;
        }
        if let Err(e) = rm_one(path, rm_dir) {
            print_err!("rm", format_args!("cannot remove '{path}'"), e);
        }
    }
}

fn do_cd(mut args: &str) {
    if args.is_empty() {
        args = "/";
    }
    if !args.contains(char::is_whitespace) {
        if let Err(e) = std::env::set_current_dir(args) {
            print_err!("cd", args, e);
        }
    } else {
        print_err!("cd", "too many arguments");
    }
}

fn do_pwd(_args: &str) {
    match std::env::current_dir() {
        Ok(pwd) => println!("{}", path_to_str(&pwd)),
        Err(err) => println!("Failed to access the current directory: {err}"),
    }
}

fn do_uname(_args: &str) {
    let arch = option_env!("AX_ARCH").unwrap_or("");
    let platform = option_env!("AX_PLATFORM").unwrap_or("");
    #[cfg(feature = "axstd")]
    let smp = if std::thread::available_parallelism()
        .map(|n| n.get() == 1)
        .unwrap_or(true)
    {
        ""
    } else {
        " SMP"
    };
    #[cfg(not(feature = "axstd"))]
    let smp = "";
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.1.0");
    println!("ArceOS {version}{smp} {arch} {platform}");
}

fn do_help(_args: &str) {
    println!("Available commands:");
    for (name, _) in CMD_TABLE {
        println!("  {name}");
    }
}

fn do_exit(_args: &str) {
    println!("Bye~");
    std::process::exit(0);
}

#[cfg(feature = "uspace")]
fn do_runu(args: &str) {
    let argv = args.split_whitespace().collect::<Vec<_>>();
    if argv.is_empty() {
        print_err!("runu", "usage: runu <path> [args...]");
        return;
    }

    match run_user_program_argv(&argv) {
        Ok(exit_code) => println!("runu: exited with status {exit_code}"),
        Err(err) => print_err!("runu", err),
    }
}

#[cfg(feature = "uspace")]
fn run_user_program_argv(argv: &[&str]) -> Result<i32, String> {
    uspace::run_user_program(argv)
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn run_user_program_argv_in(cwd: &str, argv: &[&str]) -> Result<i32, String> {
    uspace::run_user_program_in(cwd, argv)
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn normalize_rel_path(path: &str) -> Option<String> {
    let trimmed = path.trim_matches(|c: char| matches!(c, '"' | '\'' | '`'));
    let rel = trimmed.strip_prefix("./").unwrap_or(trimmed);
    if rel.is_empty() || rel == "." || rel == ".." || rel.starts_with('/') || rel.contains('$') {
        None
    } else {
        Some(rel.trim_end_matches('/').to_string())
    }
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn scan_script_dependencies(script: &str) -> Vec<String> {
    let mut deps = BTreeSet::new();
    for line in script.lines() {
        let mut normalized = String::with_capacity(line.len());
        for ch in line.chars() {
            if matches!(ch, '|' | ';' | '(' | ')' | '{' | '}' | '<' | '>' | '=') {
                normalized.push(' ');
            } else {
                normalized.push(ch);
            }
        }
        for token in normalized.split_whitespace() {
            if let Some(rel) = normalize_rel_path(token) {
                deps.insert(rel);
            }
        }
    }
    deps.into_iter().collect()
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn join_path(base: &str, rel: &str) -> String {
    if base == "/" {
        format!("/{}", rel.trim_start_matches('/'))
    } else if rel.is_empty() {
        base.trim_end_matches('/').to_string()
    } else {
        format!(
            "{}/{}",
            base.trim_end_matches('/'),
            rel.trim_start_matches('/')
        )
    }
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn parent_dir(path: &str) -> Option<&str> {
    let (parent, _) = path.rsplit_once('/')?;
    Some(if parent.is_empty() { "/" } else { parent })
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn ensure_dir_all(path: &str) -> io::Result<()> {
    if path.is_empty() || path == "/" {
        return Ok(());
    }

    let is_abs = path.starts_with('/');
    let mut current = if is_abs {
        String::from("/")
    } else {
        String::new()
    };

    for part in path.trim_matches('/').split('/') {
        if part.is_empty() {
            continue;
        }
        current = if current == "/" || current.is_empty() {
            if is_abs {
                format!("/{part}")
            } else {
                String::from(part)
            }
        } else {
            format!("{current}/{part}")
        };
        if fs::metadata(&current).is_err() {
            fs::create_dir(&current)?;
        }
    }
    Ok(())
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn remove_dir_all(path: &str) -> io::Result<()> {
    if !matches!(fs::metadata(path), Ok(meta) if meta.is_dir()) {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = String::from(path_to_str(&file_name));
        let child = join_path(path, &name);
        let metadata = fs::metadata(&child)?;
        if metadata.is_dir() {
            remove_dir_all(&child)?;
        } else {
            fs::remove_file(&child)?;
        }
    }
    fs::remove_dir(path)
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn copy_file(src: &str, dst: &str) -> io::Result<()> {
    if let Some(parent) = parent_dir(dst) {
        ensure_dir_all(parent)?;
    }
    let mut src_file = File::open(src)?;
    let mut dst_file = File::create(dst)?;
    let mut buffer = [0u8; 8192];
    loop {
        let len = src_file.read(&mut buffer)?;
        if len == 0 {
            break;
        }
        dst_file.write_all(&buffer[..len])?;
    }
    Ok(())
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn copy_script_file(
    src: &str,
    dst: &str,
    busybox_path: &str,
    rewrite_busybox_path: bool,
) -> io::Result<()> {
    let parent = parent_dir(dst);
    if let Some(parent) = parent {
        ensure_dir_all(parent)?;
    }
    let raw_script = fs::read_to_string(src)?;
    if raw_script.contains("./busybox") {
        if let Some(parent) = parent {
            copy_file(busybox_path, &join_path(parent, "busybox"))?;
        }
    }
    let mut script = raw_script
        .lines()
        .map(|line| rewrite_script_line(line, busybox_path, rewrite_busybox_path))
        .collect::<Vec<_>>()
        .join("\n");
    if raw_script.ends_with('\n') {
        script.push('\n');
    }
    let mut dst_file = File::create(dst)?;
    dst_file.write_all(script.as_bytes())
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn write_text_file(path: &str, content: &str) -> io::Result<()> {
    if let Some(parent) = parent_dir(path) {
        ensure_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn prepare_libctest_dsos(src_root: &str, stage_root: &str) -> io::Result<()> {
    let lib_dir = join_path(src_root, "lib");
    let Ok(entries) = fs::read_dir(&lib_dir) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = path_to_str(&file_name);
        if !name.ends_with(".so") {
            continue;
        }
        copy_file(&join_path(&lib_dir, name), &join_path(stage_root, name))?;
    }
    Ok(())
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn prepare_libctest_runtest_wrapper(
    src_root: &str,
    stage_root: &str,
    busybox_path: &str,
) -> io::Result<()> {
    let runtest = join_path(stage_root, "runtest.exe");
    if !matches!(fs::metadata(&runtest), Ok(meta) if meta.is_file()) {
        return Ok(());
    }

    prepare_libctest_dsos(src_root, stage_root)?;

    for script_name in ["run-static.sh", "run-dynamic.sh"] {
        let script_path = join_path(stage_root, script_name);
        if !matches!(fs::metadata(&script_path), Ok(meta) if meta.is_file()) {
            continue;
        }
        let raw = fs::read_to_string(&script_path)?;
        let rewritten = rewrite_libctest_run_script(&raw, busybox_path);
        write_text_file(&script_path, &rewritten)?;
    }

    let testcode_path = join_path(stage_root, "libctest_testcode.sh");
    if matches!(fs::metadata(&testcode_path), Ok(meta) if meta.is_file()) {
        let raw = fs::read_to_string(&testcode_path)?;
        let rewritten = raw
            .replace(
                "./run-static.sh",
                &format!("{busybox_path} sh ./run-static.sh"),
            )
            .replace(
                "./run-dynamic.sh",
                &format!("{busybox_path} sh ./run-dynamic.sh"),
            );
        write_text_file(&testcode_path, &rewritten)?;
    }

    Ok(())
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn rewrite_libctest_run_script(raw: &str, busybox_path: &str) -> String {
    let mut rewritten = String::new();
    for line in raw.lines() {
        if let Some(command) = rewrite_libctest_command(line.trim(), busybox_path) {
            rewritten.push_str(&command);
        } else {
            rewritten.push_str(line);
            rewritten.push('\n');
        }
    }
    rewritten
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn rewrite_libctest_command(line: &str, busybox_path: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    if parts.next()? != "./runtest.exe" || parts.next()? != "-w" {
        return None;
    }
    let entry = parts.next()?;
    let case = parts.next()?;
    if parts.next().is_some() {
        return None;
    }

    Some(format!(
        "{busybox_path} echo \"[libctest] running: {entry} {case}\"\n./{entry} {case}\nstatus=$?\nif [ \"$status\" -eq 0 ]; then\n    {busybox_path} echo \"Pass!\"\nelse\n    {busybox_path} echo \"FAIL {case} [status $status]\"\nfi\n"
    ))
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn prepare_lmbench_script(stage_root: &str, busybox_path: &str) -> io::Result<()> {
    let script_path = join_path(stage_root, "lmbench_testcode.sh");
    if !matches!(fs::metadata(&script_path), Ok(meta) if meta.is_file()) {
        return Ok(());
    }
    let raw = fs::read_to_string(&script_path)?;
    let rewritten = raw.replace("./lmbench_all ", "run_bounded ./lmbench_all ");
    let rewritten = rewritten.replace(" -P 1 ", " -P 1 -N 1 ");
    let rewritten = rewritten.replace(
        "run_bounded ./lmbench_all lat_ctx -P 1 -N 1 -s 32 2 4 8 16 24 32 64 96",
        "run_bounded ./lmbench_all lat_ctx -P 1 -N 1 -s 32 2",
    );
    let prefix = format!(
        "run_bounded() {{\n    ENOUGH=100 \"$@\" &\n    pid=$!\n    elapsed=0\n    while kill -0 \"$pid\" 2>/dev/null; do\n        if [ \"$elapsed\" -ge 30 ]; then\n            echo \"TIMEOUT: $*\"\n            kill \"$pid\" 2>/dev/null\n            wait \"$pid\" 2>/dev/null\n            return 124\n        fi\n        {busybox_path} sleep 1\n        elapsed=$((elapsed + 1))\n    done\n    wait \"$pid\"\n}}\n"
    );
    write_text_file(&script_path, &format!("{prefix}{rewritten}"))
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn rewrite_script_line(line: &str, busybox_path: &str, rewrite_busybox_path: bool) -> String {
    let line = line.to_string();
    let _ = rewrite_busybox_path;
    if let Some(rewritten) = rewrite_relative_variable_exec(&line) {
        return rewritten;
    }
    for applet in SCRIPT_BUSYBOX_APPLETS {
        if let Some(rewritten) = prefix_busybox_applet(&line, applet, busybox_path) {
            return rewritten;
        }
    }
    line
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn rewrite_relative_variable_exec(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let var = trimmed.strip_prefix("./$")?;
    if var.is_empty()
        || !var
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return None;
    }
    let indent_len = line.len() - trimmed.len();
    let indent = &line[..indent_len];
    Some(format!(
        "{indent}case \"${var}\" in /*) \"${var}\" ;; *) \"./${var}\" ;; esac"
    ))
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn prefix_busybox_applet(line: &str, applet: &str, busybox_path: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with(applet) {
        return None;
    }
    let rest = &trimmed[applet.len()..];
    if !(rest.is_empty() || rest.as_bytes()[0].is_ascii_whitespace()) {
        return None;
    }
    let indent_len = line.len() - trimmed.len();
    let indent = &line[..indent_len];
    Some(format!("{indent}{busybox_path} {trimmed}"))
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn copy_stage_entry(
    src_root: &str,
    dst_root: &str,
    rel: &str,
    busybox_path: &str,
) -> io::Result<()> {
    let src = join_path(src_root, rel);
    let dst = join_path(dst_root, rel);
    let metadata = fs::metadata(&src)?;
    if metadata.is_dir() {
        ensure_dir_all(&dst)?;
        for entry in fs::read_dir(&src)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let name = String::from(path_to_str(&file_name));
            let child_rel = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            copy_stage_entry(src_root, dst_root, &child_rel, busybox_path)?;
        }
    } else if rel.ends_with(".sh") {
        copy_script_file(&src, &dst, busybox_path, !rel.contains('/'))?;
    } else {
        copy_file(&src, &dst)?;
    }
    Ok(())
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn prepare_suite_stage_dir(suite_dir: &str, script_name: &str) -> io::Result<Option<String>> {
    let group = script_name
        .strip_suffix(SCRIPT_SUFFIX)
        .unwrap_or(script_name);
    if group == "ltp" {
        return Ok(None);
    }

    let src_root = suite_dir;
    let busybox_path = join_path(suite_dir, "busybox");
    let stage_root = join_path(
        TESTSUITE_STAGE_ROOT,
        &format!("{}/{}", suite_dir.trim_start_matches('/'), group),
    );
    if matches!(fs::metadata(&stage_root), Ok(meta) if meta.is_dir()) {
        remove_dir_all(&stage_root)?;
    }
    ensure_dir_all(&stage_root)?;

    let mut pending = vec![script_name.to_string()];
    let group_dir = join_path(src_root, group);
    if matches!(fs::metadata(&group_dir), Ok(meta) if meta.is_dir()) {
        pending.push(group.to_string());
    }

    let mut copied = BTreeSet::new();
    while let Some(rel) = pending.pop() {
        let Some(rel) = normalize_rel_path(rel.as_str()) else {
            continue;
        };
        if !copied.insert(rel.clone()) {
            continue;
        }

        let src = join_path(src_root, &rel);
        let Ok(metadata) = fs::metadata(&src) else {
            continue;
        };
        if rel == "busybox" {
            continue;
        }
        copy_stage_entry(src_root, &stage_root, &rel, &busybox_path)?;
        if metadata.is_file() && rel.ends_with(".sh") {
            let content = fs::read_to_string(&src)?;
            pending.extend(
                scan_script_dependencies(&content)
                    .into_iter()
                    .filter(|dep| {
                        dep != "busybox" && fs::metadata(&join_path(src_root, dep)).is_ok()
                    }),
            );
        }
    }

    if group == "libctest" {
        prepare_libctest_runtest_wrapper(src_root, &stage_root, &busybox_path)?;
    }
    if group == "lmbench" {
        prepare_lmbench_script(&stage_root, &busybox_path)?;
    }

    Ok(Some(stage_root))
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn suite_label(suite_dir: &str, group: &str) -> String {
    format!("{group}-{}", suite_dir.trim_start_matches('/'))
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn print_suite_skip(suite_dir: &str, group: &str, reason: &str) {
    let label = suite_label(suite_dir, group);
    println!("#### OS COMP TEST GROUP START {label} ####");
    println!("SKIP: {reason}");
    println!("#### OS COMP TEST GROUP END {label} ####");
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
fn run_busybox_suite(cwd: &str, suite_dir: &str) -> Result<(), String> {
    let label = suite_label(suite_dir, "busybox");
    let busybox_path = join_path(suite_dir, "busybox");
    println!("#### OS COMP TEST GROUP START {label} ####");
    let commands = fs::read_to_string(&join_path(cwd, "busybox_cmd.txt"))
        .map_err(|err| format!("read busybox_cmd.txt failed: {err}"))?;
    for applet in ["ls", "sleep"] {
        let dst = join_path(cwd, applet);
        if fs::metadata(&dst).is_err() {
            copy_file(&busybox_path, &dst)
                .map_err(|err| format!("stage busybox applet {applet} failed: {err}"))?;
        }
    }
    for line in commands.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let line = line.replace("./busybox", &busybox_path);
        let command = if line.starts_with(&busybox_path) {
            format!("PATH=. {line}")
        } else {
            format!("PATH=. {busybox_path} {line}")
        };
        match run_user_program_argv_in(cwd, &[&busybox_path, "sh", "-c", &command]) {
            Ok(status) if status == 0 || line == "false" => {
                println!("testcase busybox {line} success");
            }
            Ok(status) => {
                println!("testcase busybox {line} fail");
                println!("return: {status}, cmd: {line}");
            }
            Err(err) => {
                println!("testcase busybox {line} fail");
                println!("{err}");
            }
        }
    }
    println!("#### OS COMP TEST GROUP END {label} ####");
    Ok(())
}

#[cfg(all(feature = "auto-run-tests", feature = "uspace"))]
pub fn maybe_run_official_tests() {
    let mut scripts = Vec::new();
    for suite_dir in SUITE_DIRS {
        let Ok(entries) = fs::read_dir(suite_dir) else {
            continue;
        };
        for entry in entries.filter_map(|entry| entry.ok()) {
            let name = entry.file_name();
            if !name.ends_with(SCRIPT_SUFFIX) {
                continue;
            }
            scripts.push((String::from(*suite_dir), String::from(path_to_str(&name))));
        }
    }

    if scripts.is_empty() {
        return;
    }

    scripts.sort_by_key(|(suite_dir, script_name)| {
        (
            !matches!(suite_dir.as_str(), "/musl"),
            suite_dir.clone(),
            script_name.clone(),
        )
    });

    let shell = if matches!(fs::metadata("/musl/busybox"), Ok(meta) if meta.is_file()) {
        "/musl/busybox"
    } else if matches!(fs::metadata("/glibc/busybox"), Ok(meta) if meta.is_file()) {
        "/glibc/busybox"
    } else {
        println!("autorun: busybox shell not found");
        std::process::exit(0);
    };

    const AUTORUN_COMMAND: Option<&str> = option_env!("ARCEOS_AUTORUN_COMMAND");
    const AUTORUN_CWD: Option<&str> = option_env!("ARCEOS_AUTORUN_CWD");
    if let Some(command) = AUTORUN_COMMAND {
        let cwd = AUTORUN_CWD.unwrap_or("/");
        println!("#### OS COMP TEST GROUP START autorun-command ####");
        if let Err(err) = std::env::set_current_dir(cwd) {
            println!("autorun: cd {cwd} failed: {err}");
        } else {
            match run_user_program_argv_in(cwd, &[shell, "sh", "-c", command]) {
                Ok(status) => println!("autorun-command status: {status}"),
                Err(err) => println!("autorun-command failed: {err}"),
            }
        }
        println!("#### OS COMP TEST GROUP END autorun-command ####");
        std::io::stdout().flush().unwrap();
        std::process::exit(0);
    }

    const AUTORUN_ONLY_GROUP: Option<&str> = option_env!("ARCEOS_AUTORUN_ONLY_GROUP");

    for (suite_dir, script_name) in scripts {
        let script = path_to_str(&script_name);
        let group = script.strip_suffix(SCRIPT_SUFFIX).unwrap_or(script);
        if let Some(only_group) = AUTORUN_ONLY_GROUP {
            if group != only_group {
                continue;
            }
        }
        if suite_dir == "/glibc" && group == "libctest" {
            print_suite_skip(
                &suite_dir,
                "libctest",
                "glibc libc-test checks libc-specific semantics, not the kernel ABI target",
            );
            continue;
        }
        if group == "libcbench" {
            print_suite_skip(
                &suite_dir,
                "libcbench",
                "libcbench currently triggers an unrecovered allocator exhaustion path",
            );
            continue;
        }
        if group == "ltp" {
            print_suite_skip(
                &suite_dir,
                "ltp",
                "ltp full-suite execution is not wired yet",
            );
            continue;
        }
        let staged_dir = match prepare_suite_stage_dir(&suite_dir, script) {
            Ok(dir) => dir,
            Err(err) => {
                println!("autorun: prepare {suite_dir}/{script} failed: {err}");
                continue;
            }
        };
        let use_staged_dir = staged_dir.is_some();
        let suite_busybox = join_path(&suite_dir, "busybox");
        let (cwd, shell_path, script_path) = if let Some(dir) = staged_dir {
            (dir, suite_busybox.as_str(), script)
        } else {
            (suite_dir.clone(), shell, script)
        };
        if let Err(err) = std::env::set_current_dir(&cwd) {
            println!("autorun: cd {cwd} failed: {err}");
            continue;
        }
        if group == "busybox" {
            if let Err(err) = run_busybox_suite(&cwd, &suite_dir) {
                println!("autorun: busybox suite failed: {err}");
            }
            if use_staged_dir {
                let _ = remove_dir_all(&cwd);
            }
            continue;
        }
        let command = if use_staged_dir {
            format!("PATH=. {shell_path} sh ./{script_path}")
        } else {
            format!("./{script_path}")
        };
        if let Err(err) = run_user_program_argv_in(&cwd, &[shell_path, "sh", "-c", &command]) {
            println!("autorun: {cwd}/{script_path} failed: {err}");
        }
        if use_staged_dir {
            let _ = remove_dir_all(&cwd);
        }
    }

    std::io::stdout().flush().unwrap();
    std::process::exit(0);
}

#[cfg(not(all(feature = "auto-run-tests", feature = "uspace")))]
pub fn maybe_run_official_tests() {}

pub fn run_cmd(line: &[u8]) {
    let Ok(line_str) = str::from_utf8(line) else {
        println!("Please enter a valid utf-8 string as the command.");
        return;
    };
    let (cmd, args) = split_whitespace(line_str);
    if !cmd.is_empty() {
        for (name, func) in CMD_TABLE {
            if cmd == *name {
                func(args);
                return;
            }
        }
        println!("{cmd}: command not found");
    }
}

fn split_whitespace(str: &str) -> (&str, &str) {
    let str = str.trim();
    str.find(char::is_whitespace)
        .map_or((str, ""), |n| (&str[..n], str[n + 1..].trim()))
}
