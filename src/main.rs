use linefeed::{Interface, ReadResult};

mod types;

fn main() {
    let reader = match Interface::new("my-application") {
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
