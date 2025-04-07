use clap::Parser;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead};
use std::fs::OpenOptions;
use std::io::Write;

use serde::{Deserialize, Serialize};
// use serde_json::Result;

#[derive(Serialize, Deserialize)]
struct MemoryConfig {
    name: String,
    origin: String,
    length: String,
    attribute: String
}

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
    /// Enable JSON format output
    #[arg(short = 'j', long, action = clap::ArgAction::SetTrue)]
    json: bool,

    /// MAP_FILE_PATH to parse
    #[arg(value_name = "MAP_FILE_PATH")]
    path: String,

    /// OUTPUT_PATH(Optional) default is empty for current path
    #[arg(short = 'o', long, value_name = "OUTPUT_PATH", default_value = "")]
    output_path: Option<String>,
   
}

fn main() {
    let args = Args::parse();

    if !fs::metadata(&args.path).is_ok() {
        eprintln!("Error: File at path '{}' does not exist.", args.path);
        std::process::exit(1);
    }

    // Start parsing the file
    println!("Parsing file at path: {}", args.path);
    parse_map_file(&args.path, args.output_path.as_deref().unwrap_or(""), args.json);
}

fn parse_map_file(map_path: &str, out_path: &str, json: bool) {
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
    let file = match File::open(map_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error opening file {}: {}", map_path, e);
            std::process::exit(1);
        }
    };

    let reader = io::BufReader::new(file);

    let mut archive_members = Vec::new();
    let mut discarded_sections = Vec::new();
    let mut memory_configuration: Vec<MemoryConfig> = Vec::new();
    let mut linker_script_memory_map: Vec<MemoryMap> = Vec::new();

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
                if !(line.trim().is_empty()
                    || line.starts_with("*default*")
                    || line.starts_with("Name             Origin             Length             Attributes")
                ) {
                    
                    let mut split_line = line.split_whitespace();
                    let mem_config = MemoryConfig {
                        name: split_line.next().unwrap_or("").to_string(),
                        origin: split_line.next().unwrap_or("").to_string(),
                        length: split_line.next().unwrap_or("").to_string(),
                        attribute: split_line.collect::<Vec<_>>().join(" ")
                    };
                    memory_configuration.push(mem_config);
                }
            }
            ParserState::LinkerScriptMemoryMap => {
                if !(line.trim().is_empty()
                    || line.starts_with("LOAD")
                    || line.starts_with("START GROUP")
                    || line.starts_with("END GROUP")
                    || (line.starts_with(" *(") && line.ends_with(")"))
                    || line.ends_with("*)")
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
                                        if last_sub_section.name.contains(&sub_section) 
                                        && (!address.is_empty() && last_sub_section.address == address) {
                                            subsection_overlap = true;
                                        } else {
                                            last_sub_section.name.push(sub_section.clone());
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
                                        else {
                                            last_sub_section.object = object.clone();
                                        }
                                        // last_sub_section.object = object.clone();
                                    }
                                }                                
                                memory_map_parser_state = 3;
                            }

                        } else if memory_map_parser_state == 3 {
                            let mut subsection_overlap = false;

                            if let Some(last_map) = linker_script_memory_map.last_mut() {
                                if !last_map.sub_section.is_empty() {
                                    if let Some(last_sub_section) = last_map.sub_section.last_mut() {
                                        if last_sub_section.name.contains(&sub_section) 
                                        && (!address.is_empty() && last_sub_section.address == address) {
                                            subsection_overlap = true;
                                            last_sub_section.object = format!("{} {}", last_sub_section.object, object);
                                        }
                                    }
                                }

                                if !subsection_overlap {
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
        if json {
            match serde_json::to_string(&memory_configuration) {
            Ok(json) => {
                let output_file_path = "memory_configuration.json";
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

                if let Err(e) = writeln!(file, "{}", json) {
                    eprintln!("Error writing to file {}: {}", output_file_path, e);
                    return;
                }
                println!("Memory configuration saved to {}", output_file_path);
            }
            Err(e) => eprintln!("Error converting memory_configuration to JSON: {}", e),
            }
        } else {
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
                if let Err(e) = writeln!(file, "{}\t{}\t{}\t{}", memory.name, memory.origin, memory.length, memory.attribute) {
                    eprintln!("Error writing to file {}: {}", output_file_path, e);
                    return;
                }
            }
            println!("Memory configuration saved to {}", output_file_path);
        }

    }
    
    if !linker_script_memory_map.is_empty() {
        println!("\nLinker Script and Memory Map:");
        if json {
            match serde_json::to_string(&linker_script_memory_map) {
                // Ok(json) => println!("{}", json),
                Ok(json) => {
                    let output_file_path = "linker_script_memory_map.json";
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
        
                    if let Err(e) = writeln!(file, "{}", json) {
                    eprintln!("Error writing to file {}: {}", output_file_path, e);
                    return;
                    }
                    println!("Linker Script and Memory Map saved to {}", output_file_path);
                },
                Err(e) => eprintln!("Error converting linker_script_memory_map to JSON: {}", e),
            }
        } else {
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
}
