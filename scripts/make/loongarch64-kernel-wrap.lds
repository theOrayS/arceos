OUTPUT_ARCH(loongarch64)
ENTRY(_start)

PHDRS {
  text PT_LOAD FLAGS(5);
}

SECTIONS {
  . = 0x80000000;
  _start = .;

  .text : {
    *(.data)
    *(.text)
  } :text
}
