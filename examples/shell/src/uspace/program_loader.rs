use core::cmp;
use core::mem::size_of;

use axhal::paging::MappingFlags;
use axmm::AddrSpace;
use linux_raw_sys::auxvec;
use memory_addr::{PAGE_SIZE_4K, VirtAddr};
use std::string::{String, ToString};
use std::vec::Vec;
use xmas_elf::ElfFile;
use xmas_elf::header::{Machine, Type as ElfType};
use xmas_elf::program::{Flags as PhFlags, ProgramHeader, Type as PhType};

use super::linux_abi::{
    AUX_CLOCK_TICKS, AUX_PLATFORM, MAX_SCRIPT_INTERPRETER_DEPTH, USER_BRK_GROW_SIZE,
    USER_MMAP_BASE, USER_PIE_LOAD_BASE, USER_STACK_SIZE, USER_STACK_TOP,
};
use super::runtime_paths::{derive_exec_root_from_path, resolve_runtime_support_file};
use super::{BrkState, align_down, align_up, resolve_host_path, str_err, user_mapping_flags};

pub(super) struct LoadedImage {
    pub(super) entry: usize,
    pub(super) stack_ptr: usize,
    pub(super) argc: usize,
    pub(super) brk: BrkState,
    pub(super) exec_root: String,
    pub(super) exec_path: String,
}

struct PreparedProgram {
    image: Vec<u8>,
    argv: Vec<String>,
    path: String,
    exec_root: String,
}

struct ElfLoadInfo {
    load_bias: usize,
    entry: usize,
    phdr: usize,
    max_segment_end: usize,
    base: usize,
    interpreter: Option<String>,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct AuxEntry {
    key: usize,
    value: usize,
}

pub(super) fn load_program_image(
    aspace: &mut AddrSpace,
    cwd: &str,
    argv: &[&str],
) -> Result<LoadedImage, String> {
    let prepared = prepare_program(cwd, argv, 0)?;
    let elf = ElfFile::new(&prepared.image).map_err(|err| format!("invalid ELF: {err}"))?;
    let main = analyze_elf(&elf, USER_PIE_LOAD_BASE)?;

    aspace.clear();

    map_elf_image(aspace, &prepared.image, &elf, &main)?;
    let mut max_mapped_end = main.max_segment_end;
    let mut runtime_entry = main.entry;
    let mut interp_base = 0usize;

    if let Some(raw_interp) = main.interpreter.as_deref() {
        let interp_path = resolve_runtime_support_file(prepared.exec_root.as_str(), raw_interp)?;
        let interp_image = std::fs::read(interp_path.as_str())
            .map_err(|err| format!("failed to read interpreter {interp_path}: {err}"))?;
        let interp_elf =
            ElfFile::new(&interp_image).map_err(|err| format!("invalid interpreter ELF: {err}"))?;
        let interp = analyze_elf(
            &interp_elf,
            align_up(
                cmp::max(max_mapped_end + PAGE_SIZE_4K, USER_MMAP_BASE),
                PAGE_SIZE_4K,
            ),
        )?;
        map_elf_image(aspace, &interp_image, &interp_elf, &interp)?;
        max_mapped_end = cmp::max(max_mapped_end, interp.max_segment_end);
        runtime_entry = interp.entry;
        interp_base = interp.base;
    }

    let brk_start = align_up(main.max_segment_end, PAGE_SIZE_4K);
    let brk_limit = align_up(brk_start + USER_BRK_GROW_SIZE, PAGE_SIZE_4K);
    if brk_limit > USER_STACK_TOP - USER_STACK_SIZE {
        return Err("user virtual address space is too small".into());
    }

    aspace
        .map_alloc(
            VirtAddr::from(brk_start),
            brk_limit - brk_start,
            user_mapping_flags(true, true, false),
            false,
        )
        .map_err(|err| format!("failed to reserve brk area: {err}"))?;

    let stack_top = align_down(USER_STACK_TOP, PAGE_SIZE_4K);
    let stack_base = stack_top - USER_STACK_SIZE;
    aspace
        .map_alloc(
            VirtAddr::from(stack_base),
            USER_STACK_SIZE,
            user_mapping_flags(true, true, false),
            true,
        )
        .map_err(|err| format!("failed to map user stack: {err}"))?;

    let argv_refs = prepared.argv.iter().map(String::as_str).collect::<Vec<_>>();
    let stack_ptr = build_initial_stack(
        aspace,
        stack_base,
        stack_top,
        &argv_refs,
        prepared.path.as_str(),
        main.entry,
        interp_base,
        main.phdr,
        elf.header.pt2.ph_entry_size() as usize,
        elf.header.pt2.ph_count() as usize,
    )?;

    Ok(LoadedImage {
        entry: runtime_entry,
        stack_ptr,
        argc: prepared.argv.len(),
        brk: BrkState {
            start: brk_start,
            end: brk_start,
            limit: brk_limit,
            next_mmap: align_up(
                cmp::max(
                    max_mapped_end + PAGE_SIZE_4K,
                    cmp::max(brk_limit + PAGE_SIZE_4K, USER_MMAP_BASE),
                ),
                PAGE_SIZE_4K,
            ),
        },
        exec_root: prepared.exec_root,
        exec_path: prepared.path,
    })
}

fn prepare_program(cwd: &str, argv: &[&str], depth: usize) -> Result<PreparedProgram, String> {
    if argv.is_empty() {
        return Err("empty argv".into());
    }
    if depth > MAX_SCRIPT_INTERPRETER_DEPTH {
        return Err("script interpreter recursion limit exceeded".into());
    }

    let path = resolve_host_path(cwd.to_string(), argv[0])?;
    let image =
        std::fs::read(path.as_str()).map_err(|err| format!("failed to read {path}: {err}"))?;

    if let Some(next_argv) = parse_shebang_argv(path.as_str(), &image, argv)? {
        let next_refs = next_argv.iter().map(String::as_str).collect::<Vec<_>>();
        return prepare_program(cwd, &next_refs, depth + 1);
    }

    Ok(PreparedProgram {
        image,
        argv: argv.iter().map(|arg| (*arg).to_string()).collect(),
        path: path.clone(),
        exec_root: derive_exec_root_from_path(path.as_str()),
    })
}

fn parse_shebang_argv(
    script_path: &str,
    image: &[u8],
    argv: &[&str],
) -> Result<Option<Vec<String>>, String> {
    if image.len() < 2 || &image[..2] != b"#!" {
        return Ok(None);
    }

    let line_end = image
        .iter()
        .position(|&byte| byte == b'\n')
        .unwrap_or(image.len());
    let line = core::str::from_utf8(&image[2..line_end])
        .map_err(|_| format!("invalid shebang in {script_path}"))?
        .trim_end_matches('\r')
        .trim();
    if line.is_empty() {
        return Err(format!("empty shebang interpreter in {script_path}"));
    }

    let mut parts = line.split_whitespace();
    let raw_interpreter = parts.next().unwrap();
    let mut next_argv = resolve_script_interpreter(script_path, raw_interpreter)?;
    next_argv.extend(parts.map(str::to_string));
    next_argv.push(script_path.to_string());
    next_argv.extend(argv.iter().skip(1).map(|arg| (*arg).to_string()));
    Ok(Some(next_argv))
}

fn resolve_script_interpreter(
    script_path: &str,
    raw_interpreter: &str,
) -> Result<Vec<String>, String> {
    let base = script_dir(script_path);
    let resolved = resolve_host_path(base, raw_interpreter)?;
    if matches!(std::fs::metadata(&resolved), Ok(meta) if meta.is_file()) {
        return Ok(vec![resolved]);
    }

    if raw_interpreter == "/bin/sh" || raw_interpreter == "/busybox" {
        if let Some(busybox) = find_busybox_for_script(script_path) {
            return Ok(vec![busybox, "sh".into()]);
        }
    } else if raw_interpreter == "/bin/busybox" {
        if let Some(busybox) = find_busybox_for_script(script_path) {
            return Ok(vec![busybox]);
        }
    }

    Err(format!("script interpreter not found: {raw_interpreter}"))
}

fn find_busybox_for_script(script_path: &str) -> Option<String> {
    let mut candidates = Vec::new();
    match derive_exec_root_from_path(script_path).as_str() {
        "/musl" => candidates.push("/musl/busybox"),
        "/glibc" => candidates.push("/glibc/busybox"),
        _ => {}
    }
    candidates.push("/musl/busybox");
    candidates.push("/glibc/busybox");

    candidates.into_iter().find_map(|path| {
        matches!(std::fs::metadata(path), Ok(meta) if meta.is_file()).then(|| path.to_string())
    })
}

fn script_dir(path: &str) -> String {
    match path.rfind('/') {
        Some(0) | None => "/".into(),
        Some(idx) => path[..idx].to_string(),
    }
}

fn analyze_elf(elf: &ElfFile<'_>, preferred_base: usize) -> Result<ElfLoadInfo, String> {
    let elf_type = elf.header.pt2.type_().as_type();
    let expected_machine = if cfg!(target_arch = "riscv64") {
        Machine::RISC_V
    } else {
        Machine::Other(258)
    };
    if elf.header.pt2.machine().as_machine() != expected_machine {
        return Err("ELF machine does not match current architecture".into());
    }
    let mut min_load_addr: Option<usize> = None;
    let mut max_segment_end = 0usize;
    let mut interpreter = None;
    for ph in elf.program_iter() {
        match ph.get_type().map_err(str_err)? {
            PhType::Load => {
                let seg_start = align_down(ph.virtual_addr() as usize, PAGE_SIZE_4K);
                let seg_end = align_up(
                    ph.virtual_addr() as usize + ph.mem_size() as usize,
                    PAGE_SIZE_4K,
                );
                min_load_addr = Some(match min_load_addr {
                    Some(curr) => curr.min(seg_start),
                    None => seg_start,
                });
                max_segment_end = cmp::max(max_segment_end, seg_end);
            }
            PhType::Interp => interpreter = Some(read_interp_path(elf, &ph)?),
            _ => {}
        }
    }
    let Some(min_load_addr) = min_load_addr else {
        return Err("ELF has no LOAD segments".into());
    };

    let (load_bias, base) = match elf_type {
        ElfType::Executable => (0usize, 0usize),
        ElfType::SharedObject => {
            let mapped_min = align_up(cmp::max(preferred_base, min_load_addr), PAGE_SIZE_4K);
            let load_bias = mapped_min
                .checked_sub(min_load_addr)
                .ok_or_else(|| "failed to compute PIE load bias".to_string())?;
            (load_bias, load_bias)
        }
        _ => return Err("unsupported ELF type".into()),
    };

    Ok(ElfLoadInfo {
        load_bias,
        entry: load_bias + elf.header.pt2.entry_point() as usize,
        phdr: phdr_addr(elf, load_bias).unwrap_or(0),
        max_segment_end: load_bias + max_segment_end,
        base,
        interpreter,
    })
}

fn read_interp_path(elf: &ElfFile<'_>, ph: &ProgramHeader<'_>) -> Result<String, String> {
    let offset = ph.offset() as usize;
    let file_size = ph.file_size() as usize;
    let end = offset
        .checked_add(file_size)
        .ok_or_else(|| "PT_INTERP range overflow".to_string())?;
    let image = elf.input;
    if end > image.len() {
        return Err("PT_INTERP exceeds ELF image".into());
    }
    let raw = &image[offset..end];
    let path = raw.split(|byte| *byte == 0).next().unwrap_or(raw);
    let path = core::str::from_utf8(path).map_err(|_| "invalid PT_INTERP path".to_string())?;
    if path.is_empty() {
        return Err("empty PT_INTERP path".into());
    }
    Ok(path.to_string())
}

fn map_elf_image(
    aspace: &mut AddrSpace,
    image: &[u8],
    elf: &ElfFile<'_>,
    info: &ElfLoadInfo,
) -> Result<(), String> {
    for ph in elf.program_iter() {
        if ph.get_type().map_err(str_err)? == PhType::Load {
            map_load_segment(aspace, image, &ph, info.load_bias)?;
        }
    }
    Ok(())
}

fn map_load_segment(
    aspace: &mut AddrSpace,
    image: &[u8],
    ph: &ProgramHeader<'_>,
    load_bias: usize,
) -> Result<(), String> {
    let start = load_bias + ph.virtual_addr() as usize;
    let mem_size = ph.mem_size() as usize;
    if mem_size == 0 {
        return Ok(());
    }
    let seg_start = align_down(start, PAGE_SIZE_4K);
    let seg_end = align_up(start + mem_size, PAGE_SIZE_4K);
    let seg_size = seg_end - seg_start;
    aspace
        .map_alloc(
            VirtAddr::from(seg_start),
            seg_size,
            flags_from_ph(ph.flags()),
            true,
        )
        .map_err(|err| format!("failed to map ELF segment at {seg_start:#x}: {err}"))?;

    let file_size = ph.file_size() as usize;
    if file_size != 0 {
        let offset = ph.offset() as usize;
        let end = offset
            .checked_add(file_size)
            .ok_or_else(|| "ELF segment range overflow".to_string())?;
        if end > image.len() {
            return Err("ELF segment exceeds image size".into());
        }
        let data = &image[offset..offset + file_size];
        aspace
            .write(VirtAddr::from(start), data)
            .map_err(|err| format!("failed to write ELF segment at {start:#x}: {err}"))?;
    }
    Ok(())
}

fn phdr_addr(elf: &ElfFile<'_>, load_bias: usize) -> Option<usize> {
    let phoff = elf.header.pt2.ph_offset() as usize;
    for ph in elf.program_iter() {
        if ph.get_type().ok()? != PhType::Load {
            continue;
        }
        let seg_offset = ph.offset() as usize;
        let seg_end = seg_offset.checked_add(ph.file_size() as usize)?;
        if (seg_offset..seg_end).contains(&phoff) {
            return Some(load_bias + ph.virtual_addr() as usize + (phoff - seg_offset));
        }
    }
    None
}

fn build_initial_stack(
    aspace: &AddrSpace,
    stack_base: usize,
    stack_top: usize,
    argv: &[&str],
    execfn: &str,
    entry: usize,
    interp_base: usize,
    phdr: usize,
    phent: usize,
    phnum: usize,
) -> Result<usize, String> {
    let mut sp = stack_top;
    let random_bytes = [0x55u8; 16];
    let random_ptr = push_stack_bytes(aspace, stack_base, &mut sp, &random_bytes, 16)?;
    let mut execfn_bytes = execfn.as_bytes().to_vec();
    execfn_bytes.push(0);
    let execfn_ptr = push_stack_bytes(aspace, stack_base, &mut sp, &execfn_bytes, 1)?;
    let mut platform_bytes = AUX_PLATFORM.as_bytes().to_vec();
    platform_bytes.push(0);
    let platform_ptr = push_stack_bytes(aspace, stack_base, &mut sp, &platform_bytes, 1)?;

    let mut arg_ptrs = Vec::with_capacity(argv.len());
    for arg in argv.iter().rev() {
        let mut bytes = arg.as_bytes().to_vec();
        bytes.push(0);
        let ptr = push_stack_bytes(aspace, stack_base, &mut sp, &bytes, 1)?;
        arg_ptrs.push(ptr);
    }
    arg_ptrs.reverse();

    let aux = [
        AuxEntry {
            key: auxvec::AT_PAGESZ as usize,
            value: PAGE_SIZE_4K,
        },
        AuxEntry {
            key: auxvec::AT_UID as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_EUID as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_GID as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_EGID as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_SECURE as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_FLAGS as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_CLKTCK as usize,
            value: AUX_CLOCK_TICKS,
        },
        AuxEntry {
            key: auxvec::AT_HWCAP as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_HWCAP2 as usize,
            value: 0,
        },
        AuxEntry {
            key: auxvec::AT_PLATFORM as usize,
            value: platform_ptr,
        },
        AuxEntry {
            key: auxvec::AT_BASE_PLATFORM as usize,
            value: platform_ptr,
        },
        AuxEntry {
            key: auxvec::AT_RANDOM as usize,
            value: random_ptr,
        },
        AuxEntry {
            key: auxvec::AT_PHDR as usize,
            value: phdr,
        },
        AuxEntry {
            key: auxvec::AT_PHENT as usize,
            value: phent,
        },
        AuxEntry {
            key: auxvec::AT_PHNUM as usize,
            value: phnum,
        },
        AuxEntry {
            key: auxvec::AT_BASE as usize,
            value: interp_base,
        },
        AuxEntry {
            key: auxvec::AT_ENTRY as usize,
            value: entry,
        },
        AuxEntry {
            key: auxvec::AT_EXECFN as usize,
            value: execfn_ptr,
        },
        AuxEntry {
            key: auxvec::AT_NULL as usize,
            value: 0,
        },
    ];

    let mut words = Vec::with_capacity(1 + arg_ptrs.len() + 1 + 1 + aux.len() * 2);
    words.push(argv.len());
    words.extend(arg_ptrs.iter().copied());
    words.push(0);
    words.push(0);
    for item in aux {
        words.push(item.key);
        words.push(item.value);
    }
    let bytes = words_to_bytes(&words);
    sp = align_down(sp.saturating_sub(bytes.len()), 16);
    let end = sp + bytes.len();
    if sp < stack_base || end > stack_top {
        return Err("user stack overflow".into());
    }
    aspace
        .write(VirtAddr::from(sp), &bytes)
        .map_err(|err| format!("failed to populate user stack: {err}"))?;
    Ok(sp)
}

fn push_stack_bytes(
    aspace: &AddrSpace,
    stack_base: usize,
    sp: &mut usize,
    data: &[u8],
    align: usize,
) -> Result<usize, String> {
    *sp = align_down(sp.saturating_sub(data.len()), align.max(1));
    if *sp < stack_base {
        return Err("user stack overflow".into());
    }
    aspace
        .write(VirtAddr::from(*sp), data)
        .map_err(|err| format!("failed to write user stack data: {err}"))?;
    Ok(*sp)
}

fn words_to_bytes(words: &[usize]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(words.len() * size_of::<usize>());
    for word in words {
        bytes.extend_from_slice(&word.to_ne_bytes());
    }
    bytes
}

fn flags_from_ph(flags: PhFlags) -> MappingFlags {
    let mut out = MappingFlags::USER;
    if flags.is_read() || flags.is_execute() {
        out |= MappingFlags::READ;
    }
    if flags.is_write() {
        out |= MappingFlags::WRITE;
    }
    if flags.is_execute() {
        out |= MappingFlags::EXECUTE;
    }
    out
}
