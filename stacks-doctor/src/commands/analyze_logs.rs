use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use anyhow::{Context, Result};

/*
Run commands below to get a sample log file locally and analyze it:

```
cd stacks-blockchain/testnet/stacks-node;
cargo run -p stacks-node --bin stacks-node -- testnet 2>&1 | tee node.log;

# Set other variables accordingly
stacks-doctor -l /path/to/node.log analyze
```
*/
pub fn analyze_logs(log_file: PathBuf) -> Result<()> {
    let file = BufReader::new(File::open(log_file).context("Could not open log file")?);
    let mut is_okay = true;

    file.lines().map_while(Result::ok).for_each(|line| {
        if line.contains("mined anchored block") {
            is_okay = true;
        } else if line.contains("Failure mining") {
            is_okay = false;
            println!("Found problem in logs: {}", line);
        }
    });

    if is_okay {
        println!("No problems detected in logs");
    } else {
        println!("Problems detected in logs");
    }

    Ok(())
}
