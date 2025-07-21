use clap::{Arg, Command, ArgAction};
use std::process::{Command as StdCommand, Stdio};
use std::io::{self, Write};
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct ProcessInfo {
    port: String,
    pid: String,
    process_name: String,
    command: String,
}

fn main() {
    let matches = Command::new("apps-ports")
        .about("Find and stop applications using specific ports")
        .version("1.0")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Specific port to check")
        )
        .arg(
            Arg::new("list")
                .short('l')
                .long("list")
                .action(ArgAction::SetTrue)
                .help("List all processes using ports")
        )
        .arg(
            Arg::new("kill")
                .short('k')
                .long("kill")
                .value_name("PORT")
                .help("Kill process using the specified port")
        )
        .get_matches();

    if let Some(port) = matches.get_one::<String>("kill") {
        kill_process_by_port(port);
    } else if let Some(port) = matches.get_one::<String>("port") {
        show_process_by_port(port);
    } else if matches.get_flag("list") {
        list_all_processes();
    } else {
        list_all_processes();
    }
}

fn get_processes_using_ports() -> Vec<ProcessInfo> {
    let mut processes = Vec::new();

    // Try ss first (modern replacement for netstat)
    if let Some(ss_processes) = try_ss_command() {
        processes.extend(ss_processes);
    }

    // Try netstat as fallback
    let output = StdCommand::new("netstat")
        .args(["-tlnp"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("LISTEN") {
                if let Some(process_info) = parse_netstat_line(line) {
                    // Check if we already have this process to avoid duplicates
                    if !processes.iter().any(|p| p.pid == process_info.pid && p.port == process_info.port) {
                        processes.push(process_info);
                    }
                }
            }
        }
    }

    // Try lsof as additional fallback
    let output = StdCommand::new("lsof")
        .args(["-i", "-P", "-n", "-sTCP:LISTEN"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) { // Skip header
            if let Some(process_info) = parse_lsof_line(line) {
                // Check if we already have this process to avoid duplicates
                if !processes.iter().any(|p| p.pid == process_info.pid && p.port == process_info.port) {
                    processes.push(process_info);
                }
            }
        }
    }

    processes
}

fn parse_netstat_line(line: &str) -> Option<ProcessInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 7 {
        let address = parts[3];
        if let Some(port) = address.split(':').last() {
            let pid_info = parts[6];
            if pid_info != "-" {
                let pid_parts: Vec<&str> = pid_info.split('/').collect();
                if pid_parts.len() >= 2 {
                    let pid = pid_parts[0].to_string();
                    let process_name = pid_parts[1].to_string();
                    let command = get_command_by_pid(&pid);
                    return Some(ProcessInfo {
                        port: port.to_string(),
                        pid,
                        process_name,
                        command,
                    });
                }
            }
        }
    }
    None
}

fn parse_lsof_line(line: &str) -> Option<ProcessInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 9 {
        let process_name = parts[0].to_string();
        let pid = parts[1].to_string();
        let address = parts[8];
        
        if let Some(port_part) = address.split(':').last() {
            if let Some(port) = port_part.split('(').next() {
                let command = get_command_by_pid(&pid);
                return Some(ProcessInfo {
                    port: port.to_string(),
                    pid,
                    process_name,
                    command,
                });
            }
        }
    }
    None
}

fn get_command_by_pid(pid: &str) -> String {
    if let Ok(output) = StdCommand::new("ps")
        .args(["-p", pid, "-o", "cmd", "--no-headers"])
        .output()
    {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        "Unknown".to_string()
    }
}

fn get_process_name_by_pid(pid: &str) -> String {
    if let Ok(output) = StdCommand::new("ps")
        .args(["-p", pid, "-o", "comm", "--no-headers"])
        .output()
    {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    }
}

fn list_all_processes() {
    let processes = get_processes_using_ports();
    
    if processes.is_empty() {
        println!("No processes found using ports.");
        return;
    }

    let table = Table::new(processes);
    println!("{}", table);
}

fn show_process_by_port(port: &str) {
    let processes = get_processes_using_ports();
    let filtered: Vec<_> = processes.into_iter()
        .filter(|p| p.port == port)
        .collect();

    if filtered.is_empty() {
        println!("No process found using port {}", port);
        return;
    }

    let table = Table::new(filtered);
    println!("{}", table);
}

fn kill_process_by_port(port: &str) {
    let processes = get_processes_using_ports();
    let filtered: Vec<_> = processes.into_iter()
        .filter(|p| p.port == port)
        .collect();

    if filtered.is_empty() {
        println!("No process found using port {}", port);
        return;
    }

    println!("Found process(es) using port {}:", port);
    let table = Table::new(&filtered);
    println!("{}", table);

    for process in &filtered {
        print!("Kill process {} (PID: {})? [y/N]: ", process.process_name, process.pid);
        io::stdout().flush().unwrap();
        
        if get_user_confirmation() {
            match StdCommand::new("kill")
                .arg(&process.pid)
                .output()
            {
                Ok(_) => println!("✓ Killed process {} (PID: {})", process.process_name, process.pid),
                Err(e) => {
                    println!("✗ Failed to kill process {}: {}", process.pid, e);
                    // Try with sudo
                    print!("Try with elevated privileges? [y/N]: ");
                    io::stdout().flush().unwrap();
                    if get_user_confirmation() {
                        match StdCommand::new("sudo")
                            .args(["kill", &process.pid])
                            .output()
                        {
                            Ok(_) => println!("✓ Killed process {} (PID: {}) with sudo", process.process_name, process.pid),
                            Err(e) => println!("✗ Failed to kill process {} even with sudo: {}", process.pid, e),
                        }
                    }
                }
            }
        } else {
            println!("Skipped killing process {} (PID: {})", process.process_name, process.pid);
        }
    }
}

fn try_ss_command() -> Option<Vec<ProcessInfo>> {
    // Try ss with process info (requires elevated privileges for some processes)
    for args in [["--tcp", "--listening", "--numeric", "--processes"].as_slice(), ["--tcp", "--listening", "--numeric"].as_slice()] {
        let output = StdCommand::new("ss")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut processes = Vec::new();
            
            for line in stdout.lines().skip(1) { // Skip header
                if let Some(process_info) = parse_ss_line(line) {
                    processes.push(process_info);
                }
            }
            
            if !processes.is_empty() {
                return Some(processes);
            }
        }
    }
    None
}

fn parse_ss_line(line: &str) -> Option<ProcessInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 4 {
        let local_address = parts[3];
        
        // Extract port from address (format: *:8080 or 0.0.0.0:8080 or [::]:8080)
        let port = if let Some(colon_pos) = local_address.rfind(':') {
            local_address[colon_pos + 1..].to_string()
        } else {
            return None;
        };
        
        // Check if we have process info in the last column
        if parts.len() >= 6 {
            let process_info = parts[5];
            if process_info.contains("users:") {
                // Parse process info like: users:(("node",pid=12345,fd=10))
                if let Some(pid_start) = process_info.find("pid=") {
                    let pid_part = &process_info[pid_start + 4..];
                    if let Some(pid_end) = pid_part.find(',') {
                        let pid = pid_part[..pid_end].to_string();
                        
                        // Extract process name
                        if let Some(name_start) = process_info.find('"') {
                            if let Some(name_end) = process_info[name_start + 1..].find('"') {
                                let process_name = process_info[name_start + 1..name_start + 1 + name_end].to_string();
                                let command = get_command_by_pid(&pid);
                                
                                return Some(ProcessInfo {
                                    port,
                                    pid,
                                    process_name,
                                    command,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        // If no process info, try to find it by port using lsof
        if let Some(process_info) = find_process_by_port(&port) {
            return Some(process_info);
        }
        
        // Return basic info without process details
        return Some(ProcessInfo {
            port,
            pid: "hidden".to_string(),
            process_name: "(elevated privileges required)".to_string(),
            command: "Run with 'sudo' to see process details".to_string(),
        });
    }
    None
}

fn find_process_by_port(port: &str) -> Option<ProcessInfo> {
    // Try lsof first
    let output = StdCommand::new("lsof")
        .args(["-i", &format!(":{}", port), "-P", "-n"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            if line.contains("LISTEN") {
                return parse_lsof_line(line);
            }
        }
    }
    
    // If lsof didn't work, try to find the process using fuser
    let output = StdCommand::new("fuser")
        .args([&format!("{}/tcp", port)])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
        
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for word in stdout.split_whitespace() {
            if let Ok(pid) = word.parse::<u32>() {
                let pid_str = pid.to_string();
                let process_name = get_process_name_by_pid(&pid_str);
                let command = get_command_by_pid(&pid_str);
                
                return Some(ProcessInfo {
                    port: port.to_string(),
                    pid: pid_str,
                    process_name,
                    command,
                });
            }
        }
    }
    
    None
}

fn get_user_confirmation() -> bool {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            let input = input.trim().to_lowercase();
            input == "y" || input == "yes"
        }
        Err(_) => false,
    }
}