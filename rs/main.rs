use clap::Parser;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead};
use std::fs::OpenOptions;
use std::io::Write;


/// Simple program to analyze a map file
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Number of times to greet
    #[arg(short = 'n', long, default_value_t = 1)]
    count: u8,

    /// Path to map file
    #[arg(value_name = "PATH")]
    path: String,
}

fn main() {
    let args = Args::parse();

    for _ in 0..args.count {
        println!("Hello {}!", args.path);
    }

    if !fs::metadata(&args.path).is_ok() {
        eprintln!("Error: File at path '{}' does not exist.", args.path);
        std::process::exit(1);
    }

    // Start parsing the file
    println!("Parsing file at path: {}", args.path);
    parse_map_file(&args.path);
}

fn parse_map_file(file_path: &str) {
    // Define an enum for parser states
    #[derive(Debug)]
    enum ParserState {
        None,
        ArchiveMembers,
        DiscardedSections,
        MemoryConfiguration,
        LinkerScriptMemoryMap,
    }

    // Open the file
    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error opening file {}: {}", file_path, e);
            std::process::exit(1);
        }
    };

    let reader = io::BufReader::new(file);

    let mut archive_members = Vec::new();
    let mut discarded_sections = Vec::new();
    let mut memory_configuration = Vec::new();
    let mut linker_script_memory_map = Vec::new();
    // let mut linker_script_memory_map_buffer = Vec::new();

    let mut parser_state = ParserState::None;

    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                continue;
            }
        };

        if line.starts_with("Archive member included to satisfy reference by file") {
            println!("Found archive member: {}", line);
            parser_state = ParserState::ArchiveMembers;
            continue;
        } else if line.starts_with("Discarded input sections") {
            println!("Found discarded sections: {}", line);
            parser_state = ParserState::DiscardedSections;
            continue;
        } else if line.starts_with("Memory Configuration") {
            println!("Found memory configuration: {}", line);
            parser_state = ParserState::MemoryConfiguration;
            continue;
        } else if line.starts_with("Linker script and memory map") {
            println!("Found linker script and memory map: {}", line);
            parser_state = ParserState::LinkerScriptMemoryMap;
            continue;
        }

        match parser_state {
            ParserState::ArchiveMembers => {
                if !line.trim().is_empty() {
                    // archive_members.push(line.trim().to_string());
                    archive_members.push(line.split_whitespace().collect::<Vec<_>>().join("\t"));
                }
            }
            ParserState::DiscardedSections => {
                if !line.trim().is_empty() {
                    let temp_line;

                    if line.starts_with("                ") {
                        temp_line = if let Some(last) = discarded_sections.pop() {
                            format!("{}{}", last, line) // No need for line.to_string()
                        } else {
                            line.clone() // Use clone if you need a new String instance
                        };
                    } else {
                        temp_line = line.clone(); // Use clone if needed
                    }                 
                    discarded_sections.push(temp_line.split_whitespace().collect::<Vec<_>>().join("\t"));
                }
            }
            ParserState::MemoryConfiguration => {
                if !line.trim().is_empty() {
                    memory_configuration.push(line.split_whitespace().collect::<Vec<_>>().join("\t"));
                }
            }
            ParserState::LinkerScriptMemoryMap => {
                if !(line.trim().is_empty()
                    || line.starts_with("LOAD")
                    || line.starts_with("START GROUP")
                    || line.starts_with("END GROUP"))
                {
                    let temp_line;
                    if line.starts_with("                ")  {
                        temp_line = if let Some(last) = discarded_sections.pop() {
                            format!("{}{}", last, line) // No need for line.to_string()
                        } else {
                            line.clone() // Use clone if you need a new String instance
                        };
                    } else {
                        linker_script_memory_map_buffer.push(line.trim().to_string());
                    }
                    
                    linker_script_memory_map.push(line.trim().to_string());
                }
            }
            ParserState::None => {
                // Do nothing if no valid state is set
            }
        }
    }

    // Print parsed data
    println!("Archive Members:");
    let output_file_path = "archive_members.tsv";
    let mut file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(output_file_path)
    {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error creating file {}: {}", output_file_path, e);
            return;
        }
    };
    for member in archive_members {
        // println!("{}", member); //Debug print
        if let Err(e) = writeln!(file, "{}", member) {
            eprintln!("Error writing to file {}: {}", output_file_path, e);
            return;
        }
    }
    println!("Archive Members saved to {}", output_file_path);

    println!("\nDiscarded Sections:");
    let output_file_path = "discarded_sections.tsv";
    let mut file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(output_file_path)
    {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error creating file {}: {}", output_file_path, e);
            return;
        }
    };    
    for section in discarded_sections {
        // println!("{}", section);
        if let Err(e) = writeln!(file, "{}", section) {
            eprintln!("Error writing to file {}: {}", output_file_path, e);
            return;
        }
    }
    println!("Discarded Sections to {}", output_file_path);

    println!("\nMemory Configuration:");
    // Save memory_configuration to a plain text file with a .csv extension
    let output_file_path = "memory_configuration.tsv";
    let mut file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(output_file_path)
    {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error creating file {}: {}", output_file_path, e);
            return;
        }
    };

    for memory in memory_configuration {
        // println!("{}", memory); //Debug print
        if let Err(e) = writeln!(file, "{}", memory) {
            eprintln!("Error writing to file {}: {}", output_file_path, e);
            return;
        }
    }
    println!("Memory configuration saved to {}", output_file_path);

    // println!("\nLinker Script and Memory Map:");
    // for map in linker_script_memory_map {
    //     println!("{}", map);
    // }
}
