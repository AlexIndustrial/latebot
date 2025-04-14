use std::io::{self, BufRead, Write};

/// Starts the console interface in a separate async task
pub async fn start_console_interface() {
    tokio::spawn(async move {
        console_interface_loop().await;
    });
}

/// Main loop for the console interface
async fn console_interface_loop() {
    println!("Console interface started. Type 'help' for available commands.");
    
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut input = String::new();
    
    loop {
        input.clear();
        print!("latebot> ");
        io::stdout().flush().unwrap_or_default();
        
        if handle.read_line(&mut input).unwrap_or(0) == 0 {
            // EOF or error
            break;
        }
        
        let command = input.trim();
        
        match command {
            "test" => println!("test"),
            "help" => {
                println!("Available commands:");
                println!("  test - Test command that responds with 'test'");
                println!("  help - Show this help message");
                println!("  exit - Exit the console interface");
            },
            "exit" => {
                println!("Exiting console interface...");
                break;
            },
            "" => {}, // Ignore empty commands
            _ => println!("Unknown command: {}", command),
        }
    }
}
