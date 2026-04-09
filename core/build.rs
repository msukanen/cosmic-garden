use std::{env, fs::File, io::{BufWriter, Write}, path::Path};
use heck::ToUpperCamelCase;
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("commands.rs");
    let mut file = BufWriter::new(File::create(dest_path).unwrap());

    generate_cmd_table(&mut file, "src/cmd", "COMMANDS",
    |path| path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs"))?;

    for entry in WalkDir::new("src/cmd").min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        if !entry.path().is_dir() { continue; }
        if let Some(dir_name) = entry.path().file_name().and_then(|s| s.to_str()) {
            let table_name = format!("{}_COMMANDS", dir_name.to_uppercase());
            generate_cmd_table(&mut file, entry.path().to_str().unwrap(), &table_name,
            |path| path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs"))?;
        }
    }

    Ok(())
}

fn generate_cmd_table(file: &mut BufWriter<File>, path_str: &str, table_name: &str, filter: impl Fn(&Path) -> bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut commands = vec![];
    for entry in WalkDir::new(path_str).min_depth(1).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !filter(path) { continue; }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            // Skip common non-command files:
            if stem != "mod"
            && stem != "macros"
            && stem != "utils"
            && stem != "cmd_alias"
            {
                commands.push(stem.to_string());
            }
        }
    }

    if commands.is_empty() { return Ok(()); }

    writeln!(file, "static {}: once_cell::sync::Lazy<std::collections::HashMap<String, Box<dyn crate::cmd::Command>>> = once_cell::sync::Lazy::new(|| {{", table_name)?;
    writeln!(file, "    let mut m: std::collections::HashMap<String, Box<dyn crate::cmd::Command>> = std::collections::HashMap::new();")?;

    for cmd in &commands {
        // e.g., for "say.rs", creates `SayCommand`
        let struct_name = cmd.to_upper_camel_case();
        let module_name = if cmd == "return" {"r#return"} else {cmd};
        let full_module_path = if path_str == "src/cmd" {
            format!("{}", module_name)
        } else {
            format!("{}::{}", Path::new(path_str).file_name().unwrap().to_str().unwrap(), module_name)
        };
        writeln!(file,
            "    m.insert(\"{}\".to_string(), Box::new({}::{}Command));",
            cmd, full_module_path, struct_name
        )?;
    }
    writeln!(file, "m\n}});")?;

    Ok(())
}
