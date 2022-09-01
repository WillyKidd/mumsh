use linefeed::{Interface, ReadResult};
use libc::{signal, SIGTSTP, SIG_IGN};

mod types;
mod parser;
mod executor;

fn main() {
    unsafe {
        signal(SIGTSTP, SIG_IGN);
    }
    let reader = match Interface::new("mumsh") {
        Ok(x) => x,
        Err(e) => {
            println!("linefeed error {}", e);
            return;
        }
    };
    reader.set_prompt("mumsh $ ").unwrap();

    loop {

        match reader.read_line() {
            Ok(ReadResult::Input(line)) => {
                println!("received {}", line);
                if line.trim() == "exit" {
                    println!("bye~");
                    return;
                }
                executor::run(&line);
            },
            Ok(ReadResult::Signal(_)) => {
                println!("received signal");
            },
            Ok(ReadResult::Eof) => {
                println!("bye~");
                break;
            },
            Err(e) => {
                eprintln!("reader error {}", e);
            }
        };
    }

}
