extern crate tinyfiledialogs;

use std::{ffi::OsStr, io::{BufRead, BufReader}, path::Path, process::{Command, Stdio}};

fn main() {

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

    let filter_params: String = loop {
        match tinyfiledialogs::input_box("Update Filter Settings", "Filter settings:", "speechnorm=e=6.25:r=0.00001") {
            Some(input) => break input,
            None => (),
        }
    };

    println!("Input File: {:?}", input_file);
    println!("Output File: {:?}", output_file);
    println!("Filter params: {:?}", filter_params);

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

    let ffmpeg_args_vec = vec!("-hide_banner",  "-i", &input_file, "-b:a", "64k",  "-filter:a", &filter_params, &output_file);

    println!("Running: ffmpeg {:?}", ffmpeg_args_vec);

    exec_stream("ffmpeg", ffmpeg_args_vec.clone());
}
