use clap::Parser;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead};
use std::fs::OpenOptions;
use std::io::Write;

use serde::{Deserialize, Serialize};
use serde_json::Result;

#[derive(Serialize, Deserialize)]
struct SubSection {
    name: String,
    address: String,
    length: String,
    object: String,
    demangled: Vec<String>
}

#[derive(Serialize, Deserialize)]
struct MemoryMap {
    section: String,
    section_address: String,
    section_length: String,
    sub_section: Vec<SubSection>,
}

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
    // let mut memory_map: MemoryMap = MemoryMap {
    //     section: String::new(),
    //     section_address: String::new(),
    //     section_length: String::new(),
    //     sub_section: Vec::new()
    // };
    // println!("Memory Map: {:?}{:?}", memory_map.section, memory_map.section.is_empty());
    // println!("Memory Map: {:?}{:?}", memory_map.sub_section, memory_map.sub_section.is_empty());
    // println!("Memory Map: {:?}{:?}", memory_map.address, memory_map.address.is_empty());
    // println!("Memory Map: {:?}{:?}", memory_map.length, memory_map.length == 0);
    // println!("Memory Map: {:?}{:?}", memory_map.object, memory_map.object.is_empty());
    // println!("Memory Map: {:?}{:?}", memory_map.demangled, memory_map.demangled.is_empty());
    // let mut linker_script_memory_map_section = Vec::new();

    let mut parser_state = ParserState::None;
    let mut memory_map_parser_state = 0;

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
                    if !line.starts_with(' ') {
                        let mut memory_map = MemoryMap {
                            section: String::new(),
                            section_address: String::new(),
                            section_length: String::new(),                            
                            sub_section: Vec::new()
                        };

                        let split_line = line.split_whitespace().collect::<Vec<_>>();
                        // let split_line = line.split_whitespace();
                        // println!("1split_line: {:?}", split_line);
                        if split_line.len() == 1 {
                            memory_map.section = line.trim().to_string();

                            linker_script_memory_map.push(memory_map);
                            memory_map_parser_state = 1;
                        }
                        else if split_line.len() == 3 {
                            memory_map.section = split_line[0].to_string();
                            memory_map.section_address = split_line[1].to_string();
                            memory_map.section_length = split_line[2].to_string();

                            linker_script_memory_map.push(memory_map);
                            memory_map_parser_state = 2;
                        }
                    }
                    else if line.starts_with("                ") {
                        let split_line = line.split_whitespace().collect::<Vec<_>>();

                        if memory_map_parser_state == 1 {
                            if split_line.len() == 2 {
                                if let Some(last_memory_map) = linker_script_memory_map.last_mut() {
                                    last_memory_map.section_address = split_line[0].to_string();
                                    last_memory_map.section_length = split_line[1].to_string();
                                } else {
                                    eprintln!("Error: No memory map found to modify.");
                                }
                                memory_map_parser_state = 2;
                                // println!("1split_line: {:?}", split_line);
                            }
                        } else if memory_map_parser_state == 2 {
                            if split_line.len() == 2 {
                                if let Some(last_memory_map) = linker_script_memory_map.last_mut() {
                                    if let Some(last_sub_section) = last_memory_map.sub_section.last_mut() {
                                        last_sub_section.address = split_line[0].to_string();
                                        last_sub_section.length = split_line[1].to_string();
                                    } else {
                                        eprintln!("Error: No subsection found to modify.");
                                    }
                                } else {
                                    eprintln!("Error: No memory map found to modify.");
                                }
                                // println!("2split_line: {:?}", split_line);
                            } else if split_line.len() >= 3 {
                                if let Some(last_memory_map) = linker_script_memory_map.last_mut() {
                                    if let Some(last_sub_section) = last_memory_map.sub_section.last_mut() {
                                        last_sub_section.address = split_line[0].to_string();
                                        last_sub_section.length = split_line[1].to_string();
                                        last_sub_section.object = split_line[2..].join(" ");
                                    } else {
                                        eprintln!("Error: No subsection found to modify.");
                                    }
                                } else {
                                    eprintln!("Error: No memory map found to modify.");
                                }                                
                                // println!("2split_line: {:?}", split_line);
                            }
                        } else if memory_map_parser_state == 3 {
                            if split_line.len() >= 2 {
                                if let Some(last_memory_map) = linker_script_memory_map.last_mut() {
                                    if let Some(last_sub_section) = last_memory_map.sub_section.last_mut() {
                                        last_sub_section.demangled.push(split_line[1..].join(" "));
                                    } else {
                                        eprintln!("Error: No subsection found to modify.");
                                    }
                                } else {
                                    eprintln!("Error: No memory map found to modify.");
                                }
                                // println!("3split_line: {:?}", split_line);
                            }
                        }
                    }
                    else if !line.starts_with(" *(") {
                        let split_line = line.split_whitespace().collect::<Vec<_>>();
                        if split_line.len() == 1 {
                            if let Some(last_memory_map) = linker_script_memory_map.last_mut() {
                                let new_sub_section = SubSection {
                                    name: split_line[0].to_string(),
                                    address: String::new(),
                                    length: String::new(),
                                    object: String::new(),
                                    demangled: vec![],
                                };
                                last_memory_map.sub_section.push(new_sub_section);
                                memory_map_parser_state = 2;
                                // println!("2split_line: {:?}", split_line);
                            } else {
                                eprintln!("Error: No memory map found to add subsection.");
                            }
                        } else if split_line.len() == 3 {
                            if let Some(last_memory_map) = linker_script_memory_map.last_mut() {
                                let new_sub_section = SubSection {
                                    name: split_line[0].to_string(),
                                    address: split_line[1].to_string(),
                                    length: split_line[2].to_string(),
                                    object: String::new(),
                                    demangled: vec![],
                                };
                                last_memory_map.sub_section.push(new_sub_section);
                                memory_map_parser_state = 3;
                                // println!("2split_line: {:?}", split_line);
                            } else {
                                eprintln!("Error: No memory map found to add subsection.");
                            }
                        } else if split_line.len() >= 4 {
                            if let Some(last_memory_map) = linker_script_memory_map.last_mut() {
                                let new_sub_section = SubSection {
                                    name: split_line[1].to_string(),
                                    address: split_line[1].to_string(),
                                    length: split_line[2].to_string(),
                                    object: split_line[3..].join(" "),
                                    demangled: vec![],
                                };
                                last_memory_map.sub_section.push(new_sub_section);
                                memory_map_parser_state = 3;
                                // println!("2split_line: {:?}", split_line);
                            } else {
                                eprintln!("Error: No memory map found to add subsection.");
                            }
                        }
                    }

//                    let temp_line;
//                    if line.starts_with("                ")  {
//                        temp_line = if let Some(last) = discarded_sections.pop() {
//                            format!("{}{}", last, line) // No need for line.to_string()
//                        } else {
//                            line.clone() // Use clone if you need a new String instance
//                        };
//                    } else {
//                        linker_script_memory_map_buffer.push(line.trim().to_string());
//                    }
                    
                    // linker_script_memory_map.push(line.trim().to_string());
                }
            }
            ParserState::None => {
                // Do nothing if no valid state is set
            }
        }
    }

    // Print parsed data
    if !archive_members.is_empty() {
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
    }

    if !discarded_sections.is_empty() {
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
    } 

    if !memory_configuration.is_empty() {
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
    }

    println!("\nLinker Script and Memory Map:");
    

    // Convert linker_script_memory_map to JSON and print it
    match serde_json::to_string(&linker_script_memory_map) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error converting linker_script_memory_map to JSON: {}", e),
    }
    // for map in linker_script_memory_map {
    //     println!("--- {} {} {} ---", map.section, map.section_address, map.section_length);
    //     for sub_section in map.sub_section {
    //         println!("  {} {} {} {} {}", sub_section.name, sub_section.address, sub_section.length, sub_section.object, sub_section.demangled.join(","));
    //     }
    // }
}
