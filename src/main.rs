extern crate tinyfiledialogs;

use std::{ffi::OsStr, io::{BufRead, BufReader}, path::Path, process::{Command, Stdio}};
use tinyfiledialogs::{YesNo, MessageBoxIcon};

fn main() {

    let input_file: String = loop {
        match tinyfiledialogs::open_file_dialog("Select Input File", "", None) {
            Some(file) => break file,
            None => (),
        }
    };

    let path = Path::new(&input_file);
    let old_file_name = path.file_stem().unwrap_or(OsStr::new("")).to_str().unwrap_or("");
    let new_file_name = format!("{}-normalized", old_file_name).replace("'", "-");
    let file_extension = if path.extension().is_some() && path.extension().unwrap().len() > 0 {
        ".".to_owned() + path.extension().unwrap().to_str().unwrap_or("")
    } else {
        "".to_owned()
    };

    let is_video_default = if path.extension().unwrap_or_default().to_str().unwrap_or_default() == "mp4" {
        YesNo::Yes
    } else {
        YesNo::No
    };

    let is_video = tinyfiledialogs::message_box_yes_no("Is this a video file?", format!("Is '{}' a video file", old_file_name.to_owned() + &file_extension).replace("'", "\x60").as_str(),
                                                     MessageBoxIcon::Question, is_video_default);

    let mut new_full_file_name = new_file_name.clone() + &file_extension;

    let mut has_filename = false;
    while !has_filename {
        match tinyfiledialogs::input_box("Set output file", "Filename:", &new_full_file_name) {
            Some(input) => {
                if input.len() > 0 && path.with_file_name(new_file_name.clone() + &file_extension).exists() == false {
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
        match tinyfiledialogs::input_box("Update Filter Settings", "Filter settings:", "loudnorm,speechnorm=e=6.25:r=0.00001,loudnorm=I=-14:tp=-0.1") {
            Some(input) => break input,
            None => (),
        }
    };

    println!("Input File: {:?}", input_file);
    println!("Is Video: {:?}", is_video);
    println!("Output File: {:?}", output_file);
    println!("Filter params: {:?}", filter_params);

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
    
    let sample_rate_command;
    if let Ok(volume_command_output) = volume_command_output_result {
        println!("Finding sample rate...");
        let volume_command_lines = volume_command_output.stderr.lines();
        let sample_rate_lines: Vec<String> = volume_command_lines
            .filter(|line| line.as_ref().is_ok_and(|l| l.contains("Audio") && l.contains("Hz")))
            .map(|l| l.unwrap())
            .collect();
        if sample_rate_lines.len() > 0 {
            let stream_line = sample_rate_lines[0].clone();
            let stream_line_parts: Vec<&str> = stream_line.split(" ").collect();
            let hz_index_opt = stream_line_parts.iter().position(|&r| r == "Hz" || r == "Hz,");
            sample_rate_command = if let Some(hz_index) = hz_index_opt {
                println!("Sample Rate: {} Hz", stream_line_parts[hz_index - 1]);
                format!("-ar {}", stream_line_parts[hz_index - 1])
            } else {
                "".to_owned()
            }
        }
        else {
            sample_rate_command = "".to_owned();
        }

        println!("Finding current volumes...");
        fn get_end_of_line_containing(cmd_output: &Vec<u8>, target_string: &str) -> Option<String> {
            cmd_output.lines()
                .filter(|line| line.as_ref().is_ok_and(|l| l.contains(target_string)))
                .map(|l| l.unwrap_or("".to_owned()))
                .map(|l| l[l.find(target_string).unwrap_or(0)..].to_owned())
                .next()
                .clone()
        }

        let mean_volume_string = get_end_of_line_containing(&volume_command_output.stderr, "mean_volume");
        if mean_volume_string.is_some() {
            let mean_volume = mean_volume_string.unwrap();
            println!("Mean Volume: {}", mean_volume);
        }
        let max_volume_string = get_end_of_line_containing(&volume_command_output.stderr, "max_volume");
        if max_volume_string.is_some() {
            let max_volume = max_volume_string.unwrap();
            println!("Max Volume: {}", max_volume);
        }
    }
    else {
        sample_rate_command = "".to_owned();
        if let Err(e) = volume_command_output_result {
            println!("Error: {}", e);
        }
    }

    /*
    let ffmpeg_args = if is_video == YesNo::Yes {
        format!("ffmpeg -hide_banner -i \"{}\" -filter:a \"{}\" {} -c:v copy \"{}\"", &input_file.clone().replace("\\", "\\\\").replace("\"", "\\\""), filter_params, sample_rate_command, output_file.replace("\\", "\\\\").replace("\"", "\\\""))
    } else {
        format!("ffmpeg -hide_banner -i \"{}\" -filter:a \"{}\" {} \"{}\"", &input_file.clone().replace("\\", "\\\\").replace("\"", "\\\""), filter_params, sample_rate_command, output_file.replace("\\", "\\\\").replace("\"", "\\\""))
    };
     */

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

    let mut ffmpeg_args_vec = if is_video == YesNo::Yes {
        vec!("-hide_banner",  "-i", &input_file,  "-filter:a", &filter_params, "-c:v", "copy", &output_file)
    } else {
        vec!("-hide_banner",  "-i", &input_file,  "-filter:a", &filter_params, &output_file)
    };
    println!("Running: ffmpeg {:?}", ffmpeg_args_vec);
    let sample_rate_command_parts: Vec<&str> = sample_rate_command.split(" ").collect();
    if sample_rate_command_parts.len() > 1 {
        ffmpeg_args_vec.insert(ffmpeg_args_vec.len() - 1, sample_rate_command_parts[0]);
        ffmpeg_args_vec.insert(ffmpeg_args_vec.len() - 1, sample_rate_command_parts[1]);
    }

    exec_stream("ffmpeg", ffmpeg_args_vec.clone());
}
