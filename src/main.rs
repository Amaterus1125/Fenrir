mod audit;
mod carver;
mod common;
mod eraser;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// FENRIR - secure erase, signature-based carving, and a
/// tamper-evident audit log, built on the same checksum/atomic-write
/// primitives as the Chiral package manager (inspired from it) 
#[derive(Parser)]
#[command(name = "cforensics", version, about)]
struct Cli {
    /// Path to the audit log file (hash-chained JSONL).
    #[arg(long, global = true, default_value = "chiral-forensics-audit.log")]
    audit_log: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Securely erase a file: multi-pass overwrite + delete + verify.
    Erase {
        path: PathBuf,
        /// Use NIST 800-88 single-pass as required yk 
        #[arg(long)]
        nist: bool,
    },
    /// Securely erase every file under a folder, then remove the folder.
    EraseFolder {
        path: PathBuf,
        #[arg(long)]
        nist: bool,
    },
    /// Demo whole-device erase against a loopback image for safety as i am very scared person 
    DriveEraseDemo {
        /// Path to a loopback image file, e.g. ./demo.img
        image: PathBuf,
        #[arg(long)]
        nist: bool,
    },
    /// Signature-based file carving over a raw image / unallocated-space dump.
    Carve {
        image: PathBuf,
        #[arg(long, default_value = "carved_output")]
        out_dir: PathBuf,
    },
    /// Show the audit log.
    AuditShow,
    /// Verify the audit log's hash chain is intact.
    AuditVerify,
}

fn pattern_from_flag(nist: bool) -> eraser::ErasePattern {
    if nist {
        eraser::ErasePattern::Nist1Pass
    } else {
        eraser::ErasePattern::Dod3Pass
    }
}

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Erase { path, nist } => run_erase(path, *nist, &cli.audit_log),
        Commands::EraseFolder { path, nist } => run_erase_folder(path, *nist, &cli.audit_log),
        Commands::DriveEraseDemo { image, nist } => run_drive_erase_demo(image, *nist, &cli.audit_log),
        Commands::Carve { image, out_dir } => run_carve(image, out_dir, &cli.audit_log),
        Commands::AuditShow => audit::print_log(&cli.audit_log),
        Commands::AuditVerify => run_audit_verify(&cli.audit_log),
    };

    if let Err(e) = result {
        eprintln!("✗ {}", e);
        std::process::exit(1);
    }
}

fn run_erase(path: &PathBuf, nist: bool, log: &PathBuf) -> Result<(), String> {
    let report = eraser::erase_file(path, pattern_from_flag(nist))?;
    println!(
        " Erased {:?} — {} bytes, {} pass(es) [{}], unlinked={}, sample-verify={}",
        report.path, report.size_bytes, report.passes, report.pattern, report.verified_unlinked, report.sample_verify_ok
    );
    let details = format!(
        "{} bytes, {} passes, pattern={}, unlinked={}, sample_verify={}",
        report.size_bytes, report.passes, report.pattern, report.verified_unlinked, report.sample_verify_ok
    );
    audit::log_operation(log, "erase_file", &format!("{:?}", report.path), &details)?;
    Ok(())
}

fn run_erase_folder(path: &PathBuf, nist: bool, log: &PathBuf) -> Result<(), String> {
    let reports = eraser::erase_folder(path, pattern_from_flag(nist))?;
    println!("✓ Erased {} file(s) under {:?}", reports.len(), path);
    let details = format!("{} files erased", reports.len());
    audit::log_operation(log, "erase_folder", &format!("{:?}", path), &details)?;
    Ok(())
}

fn run_drive_erase_demo(image: &PathBuf, nist: bool, log: &PathBuf) -> Result<(), String> {
    // Same code path as erase_file — the "drive eraser" is just erase_file pointed at a loopback image instead of a physical /dev/sdX node. That's the honest scope for a demo: real device-level erase need root + raw block access, which is exactly what's on the roadmap slide.
    println!("⚠ Demo mode: erasing loopback image {:?}, not a physical device.", image);
    let report = eraser::erase_file(image, pattern_from_flag(nist))?;
    println!(
        "✓ Drive-erase demo complete — {} bytes overwritten [{}], unlinked={}, sample-verify={}",
        report.size_bytes, report.pattern, report.verified_unlinked, report.sample_verify_ok
    );
    let details = format!(
        "LOOPBACK DEMO (not physical device): {} bytes, pattern={}, unlinked={}, sample_verify={}",
        report.size_bytes, report.pattern, report.verified_unlinked, report.sample_verify_ok
    );
    audit::log_operation(log, "drive_erase_demo", &format!("{:?}", image), &details)?;
    Ok(())
}

fn run_carve(image: &PathBuf, out_dir: &PathBuf, log: &PathBuf) -> Result<(), String> {
    let hits = carver::carve_file_streaming(image, out_dir)?;
    println!(" Carved {} candidate file(s) from {:?} into {:?}", hits.len(), image, out_dir);
    for h in &hits {
        println!("   {} @ offset {} — {} bytes -> {:?}", h.kind, h.offset, h.size, h.output_path);
    }
    let details = format!("{} hits", hits.len());
    audit::log_operation(log, "carve", &format!("{:?}", image), &details)?;
    Ok(())
}

fn run_audit_verify(log: &PathBuf) -> Result<(), String> {
    let count = audit::verify_chain(log)?;
    println!(" Audit chain intact — {} entries, no tampering detected.", count);
    Ok(())
}
