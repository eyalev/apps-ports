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
    #[tabled(rename = "docker_id")]
    docker_container_id: String,
    #[tabled(rename = "docker_image")]
    docker_image: String,
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
        .arg(
            Arg::new("kill_docker_container")
                .long("kill-docker-container")
                .action(ArgAction::SetTrue)
                .help("When used with -k, kill Docker container instead of just the process")
        )
        .get_matches();

    if let Some(port) = matches.get_one::<String>("kill") {
        let kill_docker = matches.get_flag("kill_docker_container");
        kill_process_by_port(port, kill_docker);
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
                    return Some(create_process_info(
                        port.to_string(),
                        pid,
                        process_name,
                        command,
                    ));
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
                return Some(create_process_info(
                    port.to_string(),
                    pid,
                    process_name,
                    command,
                ));
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

fn get_docker_info_from_command(command: &str) -> (String, String) {
    // Check if this is a docker-proxy process
    if command.contains("docker-proxy") {
        if let Some(container_id) = extract_container_id_from_docker_proxy(command) {
            let image_name = get_container_image(&container_id);
            return (container_id, image_name);
        }
    }
    ("".to_string(), "".to_string())
}

fn get_container_image(container_id: &str) -> String {
    let output = StdCommand::new("docker")
        .args(["inspect", "-f", "{{.Config.Image}}", container_id])
        .output();
        
    if let Ok(output) = output {
        let image = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !image.is_empty() {
            return image;
        }
    }
    "unknown".to_string()
}

fn create_process_info(port: String, pid: String, process_name: String, command: String) -> ProcessInfo {
    let (docker_container_id, docker_image) = get_docker_info_from_command(&command);
    ProcessInfo {
        port,
        pid,
        process_name,
        command,
        docker_container_id,
        docker_image,
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

fn kill_process_by_port(port: &str, kill_docker: bool) {
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
        // Check if this is a docker-proxy process and we want to kill the container
        if kill_docker && process.command.contains("docker-proxy") {
            if let Some(container_id) = extract_container_id_from_docker_proxy(&process.command) {
                print!("Kill Docker container {} (running on port {})? [y/N]: ", container_id, port);
                io::stdout().flush().unwrap();
                
                if get_user_confirmation() {
                    kill_docker_container(&container_id);
                    continue;
                }
            } else {
                println!("Could not extract container ID from docker-proxy command");
            }
        }
        
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
                                
                                return Some(create_process_info(
                                    port,
                                    pid,
                                    process_name,
                                    command,
                                ));
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
        return Some(create_process_info(
            port,
            "hidden".to_string(),
            "(elevated privileges required)".to_string(),
            "Run with 'sudo' to see process details".to_string(),
        ));
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
                
                return Some(create_process_info(
                    port.to_string(),
                    pid_str,
                    process_name,
                    command,
                ));
            }
        }
    }
    
    None
}

fn extract_container_id_from_docker_proxy(command: &str) -> Option<String> {
    // Docker-proxy command format:
    // /usr/bin/docker-proxy -proto tcp -host-ip 0.0.0.0 -host-port 8080 -container-ip 172.17.0.2 -container-port 8080
    if let Some(container_ip_pos) = command.find("-container-ip ") {
        let after_container_ip = &command[container_ip_pos + 14..];
        if let Some(space_pos) = after_container_ip.find(' ') {
            let container_ip = &after_container_ip[..space_pos];
            
            // Find container ID by IP address
            return find_container_by_ip(container_ip);
        }
    }
    None
}

fn find_container_by_ip(container_ip: &str) -> Option<String> {
    let output = StdCommand::new("docker")
        .args(["ps", "--format", "{{.ID}} {{.Names}}", "--no-trunc"])
        .output();
        
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(container_id) = parts.first() {
                // Check if this container has the matching IP
                if let Some(ip) = get_container_ip(container_id) {
                    if ip == container_ip {
                        return Some(container_id.to_string());
                    }
                }
            }
        }
    }
    None
}

fn get_container_ip(container_id: &str) -> Option<String> {
    let output = StdCommand::new("docker")
        .args(["inspect", "-f", "{{range.NetworkSettings.Networks}}{{.IPAddress}}{{end}}", container_id])
        .output();
        
    if let Ok(output) = output {
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ip.is_empty() {
            return Some(ip);
        }
    }
    None
}

fn kill_docker_container(container_id: &str) {
    println!("Stopping Docker container: {}", container_id);
    
    match StdCommand::new("docker")
        .args(["stop", container_id])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                println!("✓ Successfully stopped Docker container {}", container_id);
                
                // Ask if user wants to remove the container
                print!("Remove the stopped container? [y/N]: ");
                io::stdout().flush().unwrap();
                
                if get_user_confirmation() {
                    match StdCommand::new("docker")
                        .args(["rm", container_id])
                        .output()
                    {
                        Ok(_) => println!("✓ Removed Docker container {}", container_id),
                        Err(e) => println!("✗ Failed to remove container {}: {}", container_id, e),
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("✗ Failed to stop container {}: {}", container_id, stderr);
            }
        }
        Err(e) => println!("✗ Failed to execute docker stop: {}", e),
    }
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