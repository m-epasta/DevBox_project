use colored::*;

pub fn welcome_message() {
    println!("{}", r#"
    ██████╗ ███████╗██╗   ██╗███████╗██████╗ ██╗███╗   ██╗
    ██╔══██╗██╔════╝██║   ██║██╔════╝██╔══██╗██║████╗  ██║
    ██║  ██║█████╗  ██║   ██║███████╗██████╔╝██║██╔██╗ ██║
    ██║  ██║██╔══╝  ╚██╗ ██╔╝╚════██║██╔═══╝ ██║██║╚██╗██║
    ██████╔╝███████╗ ╚████╔╝ ███████║██║     ██║██║ ╚████║
    ╚═════╝ ╚══════╝  ╚═══╝  ╚══════╝╚═╝     ╚═╝╚═╝  ╚═══╝
    "#.bright_cyan().bold());
    
    println!("{}", "               DEVSPIN              ".bright_cyan());
    println!("{}", "────────────────────────────────────────".cyan());
    println!("{}", "Quick start:".bright_white());
    println!("  {} {}", "devspin init".yellow(), "- Create a new project".white());
    println!("  {} {}", "devspin start".yellow(), "- Start services".white());
    println!("  {} {}", "devspin status".yellow(), "- Check running services".white());
    println!("  {} {}", "devspin stop".yellow(), "- Stop services".white());
    println!();
    println!("{}", "Learn more: https://devspin.dev/docs".bright_blue());
    println!();
}
