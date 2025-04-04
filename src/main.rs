use clap::Parser;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead};
use std::fs::OpenOptions;
use std::io::Write;

use serde::{Deserialize, Serialize};
// use serde_json::Result;

#[derive(Serialize, Deserialize)]
struct SubSection {
    name: Vec<String>,
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
    let mut linker_script_memory_map: Vec<MemoryMap> = Vec::new();
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
                    || line.starts_with("END GROUP")
                    || (line.starts_with(" *(") && line.ends_with(")"))
                    || line.starts_with("                                 0x")
                    || line.starts_with("OUTPUT(")
                 ) {
                    
                    let mut section = String::new();
                    let mut sub_section = String::new();
                    let mut address = String::new();
                    let mut length = String::new();
                    let mut object = String::new();

                    if line.len() < "                ".len() {
                        // Only Short section or subsection name
                        if !line.starts_with(" ") {
                            // section name
                            section = line.split_whitespace().next().unwrap_or("").to_string();
                        } else {
                            // subsection name
                            sub_section = line.split_whitespace().next().unwrap_or("").to_string();
                        }
                    } else {
                        if line["                ".len()..].starts_with("0x") 
                        || line["                ".len()..].starts_with("[!") {
                            // Address exists
                            let mut split_line = line.split_whitespace();
                            if line.chars().nth(0) != Some(' ') || line.chars().nth(1) != Some(' ') {
                                if !line.starts_with(" ") {
                                    // section name
                                    section = split_line.next().unwrap_or("").to_string();
                                } else {
                                    // subsection name
                                    sub_section = split_line.next().unwrap_or("").to_string();
                                }
                            }

                            address = split_line.next().unwrap_or("").to_string();

                            if line.len() < "*fill*         0x000002a6        0x2 ".len() {
                                if line.chars().nth("*fill*         0x000002a6        0x2 ".len() - 1) != Some(' ') {
                                    // Lenth exist
                                    length = split_line.next().unwrap_or("").to_string();
                                }
                            } else {
                                if line.chars().nth("*fill*         0x000002a6        0x2 ".len() - 1) != Some(' ') {
                                    // Lenth exist
                                    length = split_line.next().unwrap_or("").to_string();
                                }
                                if !line["*fill*         0x000002a6        0x2 ".len()..].trim().is_empty() {
                                    // object or demangled exist
                                    object = split_line.collect::<Vec<_>>().join(" ");
                                }
                            }
                        } else {
                            // Only Long section or subsection name
                            if !line.starts_with(" ") {
                                // section name
                                section = line.split_whitespace().next().unwrap_or("").to_string();
                            } else {
                                // subsection name
                                sub_section = line.split_whitespace().next().unwrap_or("").to_string();
                            }                         
                        }
                    }
                    if !section.is_empty() {
                        let memory_map  ;
                        if !address.is_empty() && !length.is_empty() {
                            memory_map = MemoryMap {
                                section: section.clone(),
                                section_address: address.clone(),
                                section_length: length.clone(),                            
                                sub_section: Vec::new()
                            };
                            memory_map_parser_state = 2;
                        } else {
                            memory_map = MemoryMap {
                                section: section.clone(),
                                section_address: String::new(),
                                section_length: String::new(),                            
                                sub_section: Vec::new()
                            };                            
                            memory_map_parser_state = 1;
                        }
                        linker_script_memory_map.push(memory_map);
                        
                    } else if !sub_section.is_empty() {
                        if memory_map_parser_state == 2 {
                            let mut subsection_overlap = false;

                            if let Some(last_map) = linker_script_memory_map.last_mut() {
                                if last_map.sub_section.is_empty() {
                                    let new_sub_section = SubSection {
                                        name: vec![sub_section.clone()],
                                        address: String::new(),
                                        length: String::new(),
                                        object: String::new(),
                                        demangled: Vec::new(),
                                    };
                                    last_map.sub_section.push(new_sub_section);
                                } else {
                                    if let Some(last_sub_section) = last_map.sub_section.last_mut() {
                                        if !last_sub_section.name.contains(&sub_section) {
                                            last_sub_section.name.push(sub_section.clone());
                                        } else {
                                            subsection_overlap = true;
                                        }
                                        // last_sub_section.name.push(sub_section.clone());
                                    }
                                }
                            }

                            if !address.is_empty() && !length.is_empty() && !object.is_empty() {
                                if let Some(last_map) = linker_script_memory_map.last_mut() {
                                    if let Some(last_sub_section) = last_map.sub_section.last_mut() {
                                        last_sub_section.address = address.clone();
                                        last_sub_section.length = length.clone();
                                        if subsection_overlap {
                                            last_sub_section.object = format!("{} {}", last_sub_section.object, object);
                                        }
                                        last_sub_section.object = object.clone();
                                    }
                                }                                
                                memory_map_parser_state = 3;
                            }

                        } else if memory_map_parser_state == 3 {
                            let mut subsection_overlap = false;
                            //TODO: as status == 2
                            if let Some(last_map) = linker_script_memory_map.last_mut() {
                                let mut new_sub_section = SubSection {
                                    name: vec![sub_section.clone()],
                                    address: String::new(),
                                    length: String::new(),
                                    object: String::new(),
                                    demangled: Vec::new(),
                                };

                                if address.is_empty() && length.is_empty() {
                                    memory_map_parser_state = 2
                                } else if !address.is_empty() && !length.is_empty() {
                                    new_sub_section.address = address.clone();
                                    new_sub_section.length = length.clone();
                                    if !object.is_empty() {
                                        new_sub_section.object = object.clone();
                                    }
                                    memory_map_parser_state = 3
                                }

                                last_map.sub_section.push(new_sub_section);
                            }

                        }
                    } else {
                        if memory_map_parser_state == 1 {
                            if !address.is_empty() && !length.is_empty() {
                                if let Some(last_map) = linker_script_memory_map.last_mut() {
                                    last_map.section_address = address.clone();
                                    last_map.section_length = length.clone();
                                    memory_map_parser_state = 2;
                                }
                            }
                        } else if memory_map_parser_state == 2 {
                            if !address.is_empty() && !length.is_empty() && !object.is_empty() {
                                if let Some(last_map) = linker_script_memory_map.last_mut() {
                                    if let Some(last_sub_section) = last_map.sub_section.last_mut() {
                                        last_sub_section.address = address.clone();
                                        last_sub_section.length = length.clone();
                                        last_sub_section.object = object.clone();
    
                                        memory_map_parser_state = 3;
                                    }
                                }
                            }
                        } else if memory_map_parser_state == 3 {
                            if !address.is_empty() && length.is_empty() && !object.is_empty() {
                                if let Some(last_map) = linker_script_memory_map.last_mut() {
                                    if let Some(last_sub_section) = last_map.sub_section.last_mut() {
                                        last_sub_section.demangled.push(object.clone());
                                    }
                                }
                            }
                        }

                    }

                    // println!("{} {} {} {} {}", section, sub_section, address, length, object);
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

    // println!("\nLinker Script and Memory Map:");
    
    // Convert linker_script_memory_map to JSON and print it
    // match serde_json::to_string(&linker_script_memory_map) {
    //     // Ok(json) => println!("{}", json),
    //     Ok(json) => {
    //         let output_file_path = "linker_script_memory_map.json";
    //         let mut file = match OpenOptions::new()
    //         .write(true)
    //         .create(true)
    //         .truncate(true)
    //         .open(output_file_path)
    //         {
    //         Ok(file) => file,
    //         Err(e) => {
    //             eprintln!("Error creating file {}: {}", output_file_path, e);
    //             return;
    //         }
    //         };

    //         if let Err(e) = writeln!(file, "{}", json) {
    //         eprintln!("Error writing to file {}: {}", output_file_path, e);
    //         return;
    //         }

    //         println!("Linker Script and Memory Map saved to {}", output_file_path);
    //     },
    //     Err(e) => eprintln!("Error converting linker_script_memory_map to JSON: {}", e),
    // }

    // for map in linker_script_memory_map {
    //     println!("--- {} {} {} ---", map.section, map.section_address, map.section_length);
    //     for sub_section in map.sub_section {
    //         println!("  {} {} {} {} {}", sub_section.name, sub_section.address, sub_section.length, sub_section.object, sub_section.demangled.join(","));
    //     }
    if !linker_script_memory_map.is_empty() {
        println!("\nLinker Script and Memory Map:");
        let output_file_path = "memory_map.tsv";
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

        for map in &linker_script_memory_map {
            let section = &map.section;
            // let section_address = &map.section_address;
            // let section_length = &map.section_length;

            for sub in &map.sub_section {
                let sub_name = sub.name.join(" ");
                let sub_address = &sub.address;
                let sub_length = &sub.length;
                let sub_object = &sub.object;
                let sub_demangled = sub.demangled.join(" ");

                if let Err(e) = writeln!(
                    file,
                    // "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    section, /*section_address, section_length,*/
                    sub_name, sub_address, sub_length, sub_object, sub_demangled
                ) {
                    eprintln!("Error writing to file {}: {}", output_file_path, e);
                    return;
                }
            }
        }
        println!("Linker Script and Memory Map saved to {}", output_file_path);
    }
}
