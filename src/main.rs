use ovmf_prebuilt::{Arch, FileType, Prebuilt, Source};
use std::env;
use std::process::{Command, exit};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("help");

    if mode == "help" || mode == "--help" || mode == "-h" {
        println!("TOGARA OS runner");
        println!("  cargo run bios   Boot the BIOS image in QEMU");
        println!("  cargo run uefi   Boot the UEFI image in QEMU");
        exit(0);
    }

    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.arg("-m")
        .arg("512M")
        .arg("-machine")
        .arg("q35")
        .arg("-serial")
        .arg("stdio")
        .arg("-no-reboot");

    match mode {
        "bios" => {
            qemu.arg("-drive")
                .arg(format!("format=raw,file={}", env!("BIOS_PATH")));
        }
        "uefi" => {
            let prebuilt = Prebuilt::fetch(Source::LATEST, "target/ovmf")
                .expect("failed to fetch OVMF firmware");
            let code = prebuilt.get_file(Arch::X64, FileType::Code);
            let vars = prebuilt.get_file(Arch::X64, FileType::Vars);
            qemu.arg("-drive")
                .arg(format!("format=raw,file={}", env!("UEFI_PATH")))
                .arg("-drive")
                .arg(format!(
                    "if=pflash,format=raw,unit=0,file={},readonly=on",
                    code.display()
                ))
                .arg("-drive")
                .arg(format!(
                    "if=pflash,format=raw,unit=1,file={},snapshot=on",
                    vars.display()
                ));
        }
        _ => {
            eprintln!("Usage: cargo run -- bios|uefi");
            exit(2);
        }
    }

    let status = qemu.status().expect("failed to start qemu-system-x86_64");
    exit(status.code().unwrap_or(1));
}
