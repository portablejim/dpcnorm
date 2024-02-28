extern crate tinyfiledialogs;

use std::{env, ffi::OsStr, fs::File, io::{BufRead, BufReader, Read, Write}, path::Path, process::{Command, Stdio}};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct DpcNormSettings {
    base_filter_params: String,
}

fn main() {

    let exe_dir_opt = env::current_exe().ok();
    let config_path_opt = if let Some(exec_dir) = exe_dir_opt { Some(exec_dir.with_file_name("dpcnorm_settings.toml")) } else { None };

    let default_settings = DpcNormSettings {
        base_filter_params: "speechnorm=e=6.25:r=0.00001".to_owned(),
    };

    if let Some(config_path) = config_path_opt.clone() {
        if !config_path.exists() {
            println!("Config file not exists: {:?}", config_path);
            let default_settings_string = toml::to_string(&default_settings).expect("TOML settings serialization failed");

            let mut config_file = File::create(config_path).expect("Failed to create config file");
            config_file.write_all(default_settings_string.as_bytes()).expect("Failed to write default settings to config file");

        }
    }

    let settings: DpcNormSettings = if let Some(config_path) = config_path_opt {
        let mut config_file = File::open(config_path).expect("Failed to open config file");
        let mut config_file_contents = String::new();
        config_file.read_to_string(&mut config_file_contents).expect("Failed to read config file");
        toml::from_str(&config_file_contents).unwrap_or(default_settings)
    } else {
        default_settings
    };

    let input_file_extensions = (["*.wav", "*.aiff"].as_ref(), "AIFF/WAV Audio Files");
    let input_file: String = loop {
        match tinyfiledialogs::open_file_dialog("Select Input File", "", Some(input_file_extensions)) {
            Some(file) => break file,
            None => (),
        }
    };

    let path = Path::new(&input_file);
    let old_file_name = path.file_stem().unwrap_or(OsStr::new("")).to_str().unwrap_or("");
    let new_file_name = format!("{}-normalized", old_file_name).replace("'", "-");
    let new_file_extension = ".mp3";

    let mut new_full_file_name = new_file_name.clone() + &new_file_extension;

    let mut has_filename = false;
    while !has_filename {
        match tinyfiledialogs::input_box("Set output file", "Filename:", &new_full_file_name) {
            Some(input) => {
                if input.len() > 0 && path.with_file_name(&input).exists() == false {
                    has_filename = true;
                    new_full_file_name = input;
                }
            },
            None => (),
        }
    }

    let output_file_path = path.with_file_name(new_full_file_name);
    let output_file = output_file_path.to_str().unwrap_or("");

    /*
    let filter_params: String = loop {
        match tinyfiledialogs::input_box("Update Filter Settings", "Filter settings:", "speechnorm=e=6.25:r=0.00001") {
            Some(input) => break input,
            None => (),
        }
    };
     */

    println!("Input File: {:?}", input_file);
    println!("Output File: {:?}", output_file);

    println!("Finding current volumes...");
    fn get_end_of_line_containing(cmd_output: &Vec<u8>, target_string: &str) -> Option<String> {
        cmd_output.lines()
            .filter(|line| line.as_ref().is_ok_and(|l| l.contains(target_string)))
            .map(|l| l.unwrap_or("".to_owned()))
            .map(|l| l[l.find(target_string).unwrap_or(0)..].to_owned())
            .next()
            .clone()
    }

    println!("Checking input file...");
    let volume_command_output_result = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-i",
            input_file.as_str(),
            "-af",
            "volumedetect",
            "-f",
            "null",
            "/dev/null"
        ])
        .output();

    let mut volume_adjustment = "0".to_owned();
    if let Ok(volume_command_output) = volume_command_output_result {
        let mean_volume_string = get_end_of_line_containing(&volume_command_output.stderr, "mean_volume");
        if mean_volume_string.is_some() {
            let mean_volume_line = mean_volume_string.unwrap();
            let mean_volume_parts: Vec<&str> = mean_volume_line.split(" ").collect();
            let mean_volume = mean_volume_parts[1];
            println!("Mean Volume: {}", mean_volume);
        }
        if let Some(max_volume_string) = get_end_of_line_containing(&volume_command_output.stderr, "max_volume") {
            let max_volume_parts: Vec<&str> = max_volume_string.split(" ").clone().collect();
            let max_volume_str = max_volume_parts.get(1).unwrap_or(&"0");
            volume_adjustment = if max_volume_str.starts_with("-") {
                (&max_volume_str[1..]).to_owned().clone()
            }
            else {
                max_volume_str.to_string()
            };
            println!("Max Volume: {}", max_volume_str);
            println!("Volume adjustment: {}", volume_adjustment);
        }
    }

    fn exec_stream<P: AsRef<Path>>(binary: P, args: Vec<&str>) {
        let mut cmd = Command::new(binary.as_ref())
            .args(&args)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
    
        {
            let stdout = cmd.stdout.as_mut().unwrap();
            let stdout_reader = BufReader::new(stdout);
            let stdout_lines = stdout_reader.lines();
    
            for line in stdout_lines {
                println!("Read: {:?}", line);
            }
        }
    
        cmd.wait().unwrap();
    }

    let volume_filter_params = format!("volume={}", volume_adjustment);
    let filter_params = volume_filter_params + "," + &settings.base_filter_params;

    println!("Filter params: {:?}", filter_params);

    let ffmpeg_args_vec = vec!("-hide_banner",  "-i", &input_file, "-b:a", "64k",  "-filter:a", &filter_params, &output_file);
    let ffmpeg_args_vec_display = ffmpeg_args_vec.clone().iter()
            .map(|s| if s.contains(" ") {
                "'".to_owned() + s + "'"
            }
            else {
                s.to_string()
            
            })
            .collect::<Vec<String>>();

    println!("Running: ffmpeg {}", ffmpeg_args_vec_display.join(" "));

    exec_stream("ffmpeg", ffmpeg_args_vec.clone());
}
