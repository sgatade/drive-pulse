mod backend;

use clap::{App, Arg, SubCommand};
use dialoguer::{Input, Select, Confirm};
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::{Editor, Context};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;
use chrono::{DateTime, Local};
use console::style;
use prettytable::{Table, Row, Cell};
use std::fs;
use drive_pulse_lib::DiffStatus;
use drive_pulse_lib::{scan_drive, compare_snapshots, save_snapshot, get_scan_history, load_snapshot};

struct PathHelper {
    completer: FilenameCompleter,
}

impl Completer for PathHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for PathHelper {
    type Hint = String;
}

impl Highlighter for PathHelper {}

impl Validator for PathHelper {}

impl Helper for PathHelper {}

fn main() {
    let matches = App::new("Drive Pulse CLI")
        .version("1.0")
        .author("Drive Pulse Team")
        .about("Manage and compare drive scans")
        .subcommand(
            SubCommand::with_name("scan")
                .about("Run a new scan")
                .arg(Arg::with_name("path")
                    .help("Path to scan (optional, will prompt if not provided)")
                    .index(1))
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("List scan history")
        )
        .subcommand(
            SubCommand::with_name("view")
                .about("View details of a scan")
                .arg(Arg::with_name("scan_id")
                    .help("ID of the scan to view (optional, will prompt if not provided)")
                    .index(1))
        )
        .subcommand(
            SubCommand::with_name("compare")
                .about("Compare two scans")
                .arg(Arg::with_name("scan1")
                    .help("ID of the first scan (optional, will prompt if not provided)")
                    .index(1))
                .arg(Arg::with_name("scan2")
                    .help("ID of the second scan (optional, will prompt if not provided)")
                    .index(2))
        )
        .subcommand(
            SubCommand::with_name("export")
                .about("Export comparison of two scans")
                .arg(Arg::with_name("scan1")
                    .help("ID of the first scan (optional, will prompt if not provided)")
                    .index(1))
                .arg(Arg::with_name("scan2")
                    .help("ID of the second scan (optional, will prompt if not provided)")
                    .index(2))
                .arg(Arg::with_name("format")
                    .help("Export format: json or csv (optional, will prompt if not provided)")
                    .index(3))
                .arg(Arg::with_name("output")
                    .short("o")
                    .long("output")
                    .help("Output file path (optional, will prompt if not provided)")
                    .takes_value(true))
        )
        .get_matches();

    let result = if let Some(matches) = matches.subcommand_matches("scan") {
        handle_scan(matches)
    } else if let Some(_) = matches.subcommand_matches("list") {
        handle_list()
    } else if let Some(matches) = matches.subcommand_matches("view") {
        handle_view(matches)
    } else if let Some(matches) = matches.subcommand_matches("compare") {
        handle_compare(matches)
    } else if let Some(matches) = matches.subcommand_matches("export") {
        handle_export(matches)
    } else {
        // Interactive mode
        handle_interactive()
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn handle_scan(matches: &clap::ArgMatches) -> Result<(), String> {
    let path = match matches.value_of("path") {
        Some(p) => p.to_string(),
        None => {
            let mut rl = Editor::new().map_err(|e| format!("Failed to create editor: {}", e))?;
            rl.set_helper(Some(PathHelper {
                completer: FilenameCompleter::new(),
            }));
            
            println!("\n{}", style("Enter path to scan (use Tab for autocomplete):").cyan());
            match rl.readline("Path: ") {
                Ok(line) => line.trim().to_string(),
                Err(ReadlineError::Interrupted) => {
                    return Err("Cancelled by user".to_string());
                }
                Err(ReadlineError::Eof) => {
                    return Err("EOF".to_string());
                }
                Err(err) => {
                    return Err(format!("Failed to read input: {}", err));
                }
            }
        }
    };

    println!("\n{} Starting scan of: {}\n", style("🔍").cyan(), style(&path).yellow().bold());
    
    let mut last_count = 0;
    let snapshot = drive_pulse_lib::scan_drive(path, |count: usize, current_path: String| {
        if count % 100 == 0 || count != last_count {
            // Truncate path if too long using character-aware slicing
            let truncated_path = if current_path.chars().count() > 60 {
                let chars: Vec<char> = current_path.chars().collect();
                let start = chars.len().saturating_sub(57);
                format!("...{}", chars[start..].iter().collect::<String>())
            } else {
                current_path.clone()
            };
            print!("\r{} Scanning... {} files found | {:<60}", 
                style("🔍").cyan(), 
                style(format!("{:6}", count)).yellow().bold(),
                style(&truncated_path).dim()
            );
            use std::io::Write;
            std::io::stdout().flush().unwrap();
            last_count = count;
        }
    })?;
    
    print!("\r{}\r", " ".repeat(150)); // Clear the line
    println!("{} Scan completed successfully!", style("✓").green().bold());
    println!();
    
    let rows = vec![
        vec![style("Snapshot ID").cyan().bold().to_string(), snapshot.id.clone()],
        vec![style("Total Files").cyan().bold().to_string(), format!("{}", snapshot.total_files)],
        vec![style("Total Size").cyan().bold().to_string(), format_size(snapshot.total_size)],
        vec![style("Duration").cyan().bold().to_string(), format!("{} seconds", snapshot.scan_duration)],
    ];
    let table = create_table_with_rows(rows);
    
    println!("{}", table);
    
    drive_pulse_lib::save_snapshot(&snapshot, false, None)?;
    
    Ok(())
}

fn handle_list() -> Result<(), String> {
    let history = drive_pulse_lib::get_scan_history()?;
    
    if history.is_empty() {
        println!("\n{} No scans found.", style("ℹ").blue());
        return Ok(());
    }
    
    println!("\n{} Scan History\n", style("📊").cyan().bold());
    
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("ID"),
        Cell::new("Drive Path"),
        Cell::new("Date"),
        Cell::new("Files"),
        Cell::new("Size"),
    ]));
    
    for scan in history {
        let datetime = DateTime::from_timestamp(scan.timestamp, 0)
            .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        table.add_row(Row::new(vec![
            Cell::new(&scan.id),
            Cell::new(&scan.drive_path),
            Cell::new(&datetime),
            Cell::new(&format!("{}", scan.total_files)),
            Cell::new(&format_size(scan.total_size)),
        ]));
    }
    
    println!("{}\n", table);
    
    Ok(())
}

fn handle_view(matches: &clap::ArgMatches) -> Result<(), String> {
    let scan_id = match matches.value_of("scan_id") {
        Some(id) => id.to_string(),
        None => {
            // Show list and let user select
            let history = drive_pulse_lib::get_scan_history()?;
            if history.is_empty() {
                return Err("No scans found.".to_string());
            }
            
            let items: Vec<String> = history.iter()
                .map(|s| format!("{} - {} ({})", s.id, s.drive_path, 
                    DateTime::from_timestamp(s.timestamp, 0)
                        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "Unknown".to_string())))
                .collect();
            
            let selection = Select::new()
                .with_prompt("Select a scan to view")
                .items(&items)
                .interact()
                .map_err(|e| format!("Failed to get selection: {}", e))?;
            
            history[selection].id.clone()
        }
    };

    let snapshot = drive_pulse_lib::load_snapshot(&scan_id, None)?;
    
    println!("\n{} Snapshot Details\n", style("📄").cyan().bold());
    
    let rows = vec![
        vec![style("ID").cyan().bold().to_string(), snapshot.id.clone()],
        vec![style("Drive Path").cyan().bold().to_string(), snapshot.drive_path.clone()],
        vec![style("Timestamp").cyan().bold().to_string(),
            DateTime::from_timestamp(snapshot.timestamp, 0)
                .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string())],
        vec![style("Total Files").cyan().bold().to_string(), format!("{}", snapshot.total_files)],
        vec![style("Total Size").cyan().bold().to_string(), format_size(snapshot.total_size)],
        vec![style("Scan Duration").cyan().bold().to_string(), format!("{} seconds", snapshot.scan_duration)],
    ];
    let table = create_table_with_rows(rows);
    
    println!("{}\n", table);
    
    let show_files = Confirm::new()
        .with_prompt("Show file list?")
        .interact()
        .map_err(|e| format!("Failed to get confirmation: {}", e))?;
    
    if show_files {
        println!("\n{} File List (showing first 100)\n", style("📁").cyan().bold());
        
        let mut table = Table::new();
        table.add_row(Row::new(vec![
            Cell::new("#"),
            Cell::new("Path"),
            Cell::new("Size"),
        ]));
        
        for (i, file) in snapshot.files.iter().take(100).enumerate() {
            table.add_row(Row::new(vec![
                Cell::new(&format!("{}", i + 1)),
                Cell::new(&file.path),
                Cell::new(&format_size(file.size)),
            ]));
        }
        
        println!("{}", table);
        
        if snapshot.files.len() > 100 {
            println!("\n{} {} more files not shown", style("...").dim(), snapshot.files.len() - 100);
        }
    }
    
    Ok(())
}

fn handle_compare(matches: &clap::ArgMatches) -> Result<(), String> {
    let history = drive_pulse_lib::get_scan_history()?;
    if history.len() < 2 {
        return Err("Need at least 2 scans to compare.".to_string());
    }
    
    let scan1_id = match matches.value_of("scan1") {
        Some(id) => id.to_string(),
        None => {
            let items: Vec<String> = history.iter()
                .map(|s| format!("{} - {} ({})", s.id, s.drive_path, 
                    DateTime::from_timestamp(s.timestamp, 0)
                        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "Unknown".to_string())))
                .collect();
            
            let selection = Select::new()
                .with_prompt("Select first scan")
                .items(&items)
                .interact()
                .map_err(|e| format!("Failed to get selection: {}", e))?;
            
            history[selection].id.clone()
        }
    };
    
    let scan2_id = match matches.value_of("scan2") {
        Some(id) => id.to_string(),
        None => {
            let items: Vec<String> = history.iter()
                .filter(|s| s.id != scan1_id)
                .map(|s| format!("{} - {} ({})", s.id, s.drive_path, 
                    DateTime::from_timestamp(s.timestamp, 0)
                        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "Unknown".to_string())))
                .collect();
            
            let selection = Select::new()
                .with_prompt("Select second scan")
                .items(&items)
                .interact()
                .map_err(|e| format!("Failed to get selection: {}", e))?;
            
            history.iter()
                .filter(|s| s.id != scan1_id)
                .nth(selection)
                .unwrap()
                .id.clone()
        }
    };

    println!("\n{} Comparing scans...\n", style("🔄").cyan());
    let snapshot1 = drive_pulse_lib::load_snapshot(&scan1_id, None)?;
    let snapshot2 = drive_pulse_lib::load_snapshot(&scan2_id, None)?;
    let comparison = drive_pulse_lib::compare_snapshots(&snapshot1, &snapshot2);
    
    println!("{} Comparison Results\n", style("📊").cyan().bold());
    
    // Snapshot info
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("ID"),
        Cell::new("Drive Path"),
        Cell::new("Date"),
        Cell::new("Files"),
        Cell::new("Size"),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("ID"),
        Cell::new(&comparison.snapshot1.id),
        Cell::new(&comparison.snapshot2.id),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Path"),
        Cell::new(&comparison.snapshot1.drive_path),
        Cell::new(&comparison.snapshot2.drive_path),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Date"),
        Cell::new(&DateTime::from_timestamp(comparison.snapshot1.timestamp, 0)
            .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string())),
        Cell::new(&DateTime::from_timestamp(comparison.snapshot2.timestamp, 0)
            .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string())),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Files"),
        Cell::new(&format!("{}", comparison.snapshot1.total_files)),
        Cell::new(&format!("{}", comparison.snapshot2.total_files)),
    ]));
    table.add_row(Row::new(vec![
        Cell::new("Size"),
        Cell::new(&format_size(comparison.snapshot1.total_size)),
        Cell::new(&format_size(comparison.snapshot2.total_size)),
    ]));
    
    println!("{}\n", table);
    
    // Changes summary
    let mut table = Table::new();
    table.add_row(Row::new(vec![
        Cell::new("ID"),
        Cell::new("Drive Path"),
        Cell::new("Date"),
        Cell::new("Files"),
        Cell::new("Size"),
    ]));
    for scan in &history {
        let datetime = DateTime::from_timestamp(scan.timestamp, 0)
            .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        table.add_row(Row::new(vec![
            Cell::new(&scan.id),
            Cell::new(&scan.drive_path),
            Cell::new(&datetime),
            Cell::new(&format!("{}", scan.total_files)),
            Cell::new(&format_size(scan.total_size)),
        ]));
    }
    table.printstd();
    let show_details = Confirm::new()
        .with_prompt("Show detailed changes?")
        .interact()
        .map_err(|e| format!("Failed to get confirmation: {}", e))?;
    
    if show_details {
        println!("\n{} Detailed Changes (showing first 50)\n", style("📝").cyan().bold());
        
        let mut table = Table::new();
        table.add_row(Row::new(vec![
            Cell::new("ID"),
            Cell::new("Drive Path"),
            Cell::new("Date"),
            Cell::new("Files"),
            Cell::new("Size"),
        ]));
        for scan in &history {
            let datetime = DateTime::from_timestamp(scan.timestamp, 0)
                .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            table.add_row(Row::new(vec![
                Cell::new(&scan.id),
                Cell::new(&scan.drive_path),
                Cell::new(&datetime),
                Cell::new(&format!("{}", scan.total_files)),
                Cell::new(&format_size(scan.total_size)),
            ]));
        }
        table.printstd();

        // Details table for diffs (example, refactor as needed)
        let mut details_table = Table::new();
        details_table.add_row(Row::new(vec![
            Cell::new("Change"),
            Cell::new("Path"),
            Cell::new("Old Size"),
            Cell::new("New Size"),
        ]));
        for diff in comparison.diffs.iter().take(50) {
            match diff.status {
                DiffStatus::Added => {
                    details_table.add_row(Row::new(vec![
                        Cell::new("Added"),
                        Cell::new(&diff.path),
                        Cell::new("-"),
                        Cell::new(&format_size(diff.new_size.unwrap_or(0))),
                    ]));
                },
                DiffStatus::Deleted => {
                    details_table.add_row(Row::new(vec![
                        Cell::new("Deleted"),
                        Cell::new(&diff.path),
                        Cell::new(&format_size(diff.old_size.unwrap_or(0))),
                        Cell::new("-"),
                    ]));
                },
                DiffStatus::Modified => {
                    details_table.add_row(Row::new(vec![
                        Cell::new("Modified"),
                        Cell::new(&diff.path),
                        Cell::new(&format_size(diff.old_size.unwrap_or(0))),
                        Cell::new(&format_size(diff.new_size.unwrap_or(0))),
                    ]));
                },
                DiffStatus::Unchanged => {},
            }
        }
        details_table.printstd();
        if comparison.diffs.len() > 50 {
            println!("\n{} {} more changes not shown", style("...").dim(), comparison.diffs.len() - 50);
        }
    }

    Ok(())
}

fn handle_export(matches: &clap::ArgMatches) -> Result<(), String> {
    let history = drive_pulse_lib::get_scan_history()?;
    if history.len() < 2 {
        return Err("Need at least 2 scans to compare.".to_string());
    }
    
    let scan1_id = match matches.value_of("scan1") {
        Some(id) => id.to_string(),
        None => {
            let items: Vec<String> = history.iter()
                .map(|s| format!("{} - {}", s.id, s.drive_path))
                .collect();
            
            let selection = Select::new()
                .with_prompt("Select first scan")
                .items(&items)
                .interact()
                .map_err(|e| format!("Failed to get selection: {}", e))?;
            
            history[selection].id.clone()
        }
    };
    
    let scan2_id = match matches.value_of("scan2") {
        Some(id) => id.to_string(),
        None => {
            let items: Vec<String> = history.iter()
                .filter(|s| s.id != scan1_id)
                .map(|s| format!("{} - {}", s.id, s.drive_path))
                .collect();
            
            let selection = Select::new()
                .with_prompt("Select second scan")
                .items(&items)
                .interact()
                .map_err(|e| format!("Failed to get selection: {}", e))?;
            
            history.iter()
                .filter(|s| s.id != scan1_id)
                .nth(selection)
                .unwrap()
                .id.clone()
        }
    };

    let format = match matches.value_of("format") {
        Some(f) => f.to_lowercase(),
        None => {
            let formats = vec!["json", "csv"];
            let selection = Select::new()
                .with_prompt("Select export format")
                .items(&formats)
                .interact()
                .map_err(|e| format!("Failed to get selection: {}", e))?;
            
            formats[selection].to_string()
        }
    };

    let output = match matches.value_of("output") {
        Some(o) => o.to_string(),
        None => {
            Input::new()
                .with_prompt("Enter output file path")
                .default(format!("comparison.{}", format))
                .interact()
                .map_err(|e| format!("Failed to get input: {}", e))?
        }
    };

    println!("\n{} Comparing scans...\n", style("🔄").cyan());
    let snapshot1 = drive_pulse_lib::load_snapshot(&scan1_id, None)?;
    let snapshot2 = drive_pulse_lib::load_snapshot(&scan2_id, None)?;
    let comparison = drive_pulse_lib::compare_snapshots(&snapshot1, &snapshot2);
    
    println!("{} Exporting to {}...", style("💾").cyan(), style(&output).yellow());
    
    match format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&comparison)
                .map_err(|e| format!("Failed to serialize: {}", e))?;
            fs::write(&output, json)
                .map_err(|e| format!("Failed to write file: {}", e))?;
        },
        "csv" => {
            let mut wtr = csv::Writer::from_path(&output)
                .map_err(|e| format!("Failed to create CSV writer: {}", e))?;
            
            wtr.write_record(&["Path", "Status", "Old Size", "New Size", "Old Modified", "New Modified"])
                .map_err(|e| format!("Failed to write CSV header: {}", e))?;
            
            for diff in &comparison.diffs {
                wtr.write_record(&[
                    &diff.path,
                    &format!("{:?}", diff.status),
                    &diff.old_size.map(|s: u64| s.to_string()).unwrap_or_default(),
                    &diff.new_size.map(|s: u64| s.to_string()).unwrap_or_default(),
                    &diff.old_modified.map(|m: i64| m.to_string()).unwrap_or_default(),
                    &diff.new_modified.map(|m: i64| m.to_string()).unwrap_or_default(),
                ]).map_err(|e| format!("Failed to write CSV record: {}", e))?;
            }
            
            wtr.flush().map_err(|e| format!("Failed to flush CSV: {}", e))?;
        },
        _ => return Err(format!("Unsupported format: {}", format)),
    }
    
    println!("\n{} Exported successfully to {}", style("✓").green().bold(), style(&output).yellow());
    
    Ok(())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;
    
    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

fn handle_interactive() -> Result<(), String> {
    println!("\n{}\n", style("Drive Pulse CLI").cyan().bold().underlined());
    
    loop {
        let options = vec![
            "Run a new scan",
            "List scan history",
            "View scan details",
            "Compare two scans",
            "Export comparison",
            "Exit",
        ];
        
        let selection = Select::new()
            .with_prompt("What would you like to do?")
            .items(&options)
            .interact()
            .map_err(|e| format!("Failed to get selection: {}", e))?;
        
        let result = match selection {
            0 => handle_scan(&clap::ArgMatches::default()),
            1 => handle_list(),
            2 => handle_view(&clap::ArgMatches::default()),
            3 => handle_compare(&clap::ArgMatches::default()),
            4 => handle_export(&clap::ArgMatches::default()),
            5 => {
                println!("\n{} Goodbye!\n", style("👋").cyan());
                return Ok(());
            },
            _ => Ok(()),
        };
        
        if let Err(e) = result {
            eprintln!("\n{} {}\n", style("✗").red().bold(), style(e).red());
        }
        
        println!(); // Add spacing between operations
    }
}

fn create_table_with_rows(rows: Vec<Vec<String>>) -> Table {
    let mut table = Table::new();
    for row in rows {
        table.add_row(Row::new(row.iter().map(|s| Cell::new(s)).collect()));
    }
    table
}

// Ensure all header strings are wrapped in `Cell::new`
fn create_table_with_header(header: Vec<&str>, rows: Vec<Vec<&str>>) -> Table {
    let mut table = Table::new();
    table.add_row(Row::new(header.into_iter().map(Cell::new).collect()));
    for row in rows {
        table.add_row(Row::new(row.into_iter().map(Cell::new).collect()));
    }
    table
}