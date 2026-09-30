//! Signs Kaiki Chat's network preset (Docs/V1_NETWORK_PRESET_2026_09_28_RU.md)
//! with the key kept offline; the app checks it with the key built in.
#[cfg(unix)]
mod preset {
    use agentic_node::network_preset::{PRESET_KEY, public_key, sign, verify};
    use clap::{Parser, Subcommand};
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::PathBuf;

    #[derive(Parser)]
    #[command(version, about = "Signs Kaiki Chat's network preset")]
    struct Cli {
        #[command(subcommand)]
        command: Command,
    }

    #[derive(Subcommand)]
    enum Command {
        /// Make a new key into SEED (0600, never overwritten); print its
        /// public key.
        Keygen { seed: PathBuf },
        /// Print the file to serve for the preset in PAYLOAD, signed with
        /// the key in SEED.
        Sign { seed: PathBuf, payload: PathBuf },
        /// Print the preset in FILE if it is signed by KEY (the app's key
        /// unless given) and valid.
        Verify {
            file: PathBuf,
            #[arg(long, default_value = PRESET_KEY)]
            key: String,
        },
    }

    fn read_seed(path: &PathBuf) -> Result<[u8; 32], String> {
        let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        hex::decode(text.trim())
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| format!("{} is not a 32-byte hex seed", path.display()))
    }

    fn run() -> Result<String, String> {
        match Cli::parse().command {
            Command::Keygen { seed } => {
                let mut bytes = [0u8; 32];
                getrandom::fill(&mut bytes).map_err(|error| error.to_string())?;
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&seed)
                    .map_err(|error| format!("{}: {error}", seed.display()))?;
                writeln!(file, "{}", hex::encode(bytes)).map_err(|error| error.to_string())?;
                Ok(public_key(&bytes))
            }
            Command::Sign { seed, payload } => {
                let seed = read_seed(&seed)?;
                let text = std::fs::read_to_string(&payload).map_err(|error| error.to_string())?;
                sign(text.trim(), &seed).map_err(|error| error.to_string())
            }
            Command::Verify { file, key } => {
                let bytes = std::fs::read(&file).map_err(|error| error.to_string())?;
                let preset = verify(&bytes, &key).map_err(|error| error.to_string())?;
                serde_json::to_string_pretty(&preset).map_err(|error| error.to_string())
            }
        }
    }

    pub fn main() -> std::process::ExitCode {
        match run() {
            Ok(text) => {
                println!("{text}");
                std::process::ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("kaiki-preset: {error}");
                std::process::ExitCode::FAILURE
            }
        }
    }
}

#[cfg(unix)]
fn main() -> std::process::ExitCode {
    preset::main()
}

#[cfg(not(unix))]
fn main() {
    eprintln!("kaiki-preset runs on unix");
}
