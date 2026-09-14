# TOGARA OS Kernel Makefile
#
# Builds kernel and creates bootable ISO with Limine bootloader

# Toolchain
RUSTC := rustc
CARGO := cargo
NASM := nasm
LD := ld

# Target
TARGET := x86_64-unknown-none
KERNEL_BIN := target/$(TARGET)/release/togara-kernel
KERNEL_ELF := target/togara-kernel.elf

# Bootloader
LIMINE_DIR := limine
LIMINE_BIN := $(LIMINE_DIR)/limine-bios.sys
LIMINE_EFI := $(LIMINE_DIR)/BOOTX64.EFI

# ISO
ISO_DIR := isodir
ISO_BIN := togara-os.iso

# QEMU
QEMU := qemu-system-x86_64

.PHONY: all clean build run iso

all: $(KERNEL_ELF) iso

# Build kernel
$(KERNEL_ELF): kernel/src/main.rs kernel/Cargo.toml
	$(CARGO) build --release --target $(TARGET).json
	cp $(KERNEL_BIN) $(KERNEL_ELF)

# Build hello world program
hello.elf: hello.asm
	$(NASM) -f elf64 hello.asm -o hello.o
	$(LD) -o hello.elf hello.o
	@rm hello.o

# Create bootable ISO
iso: $(KERNEL_ELF) hello.elf
	@mkdir -p $(ISO_DIR)/boot/limine
	@mkdir -p $(ISO_DIR)/boot/to gara
	
	# Copy kernel and modules
	cp $(KERNEL_ELF) $(ISO_DIR)/boot/togara/togara-kernel.elf
	cp hello.elf $(ISO_DIR)/boot/togara/hello.elf
	cp limine.cfg $(ISO_DIR)/boot/togara/
	
	# Copy Limine bootloader
	cp $(LIMINE_BIN) $(ISO_DIR)/boot/limine/
	cp $(LIMINE_EFI) $(ISO_DIR)/boot/limine/
	
	# Create ISO
	xorriso -as mkisofs -b boot/limine/limine-bios.sys \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot boot/limine/BOOTX64.EFI \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		$(ISO_DIR)/ -o $(ISO_BIN)
	
	@echo "Created $(ISO_BIN)"

# Run in QEMU
run: $(ISO_BIN)
	$(QEMU) -cdrom $(ISO_BIN) -serial stdio -m 512M

# Run with debug
run-debug: $(ISO_BIN)
	$(QEMU) -cdrom $(ISO_BIN) -serial stdio -m 512M -d int -no-reboot

# Clean
clean:
	$(CARGO) clean
	rm -f hello.elf hello.o
	rm -rf $(ISO_DIR)
	rm -f $(ISO_BIN)
	rm -f $(KERNEL_ELF)
