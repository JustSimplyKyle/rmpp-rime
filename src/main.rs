use std::{
    cell::LazyCell,
    collections::HashMap,
    os::unix::fs::FileTypeExt,
    path::PathBuf,
    sync::{Arc, LazyLock},
};

use anyhow::Context;
use rime_api::{
    create_session, full_deploy_and_wait, initialize, set_notification_handler, setup,
    DeployResult, Traits,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    select,
};

struct Rime {
    data_dir: PathBuf,
    user_dir: PathBuf,
    keys_path: PathBuf,
    candidates_path: PathBuf,
    backspace_path: PathBuf,
    output_path: PathBuf,
}

static KEY_MAP: LazyLock<HashMap<String, &str>> = LazyLock::new(|| {
    HashMap::from_iter([
        (String::from("ㄅ"), "1"),
        (String::from("ㄉ"), "2"),
        (String::from("ˇ"), "3"),
        (String::from("ˋ"), "4"),
        (String::from("ㄓ"), "5"),
        (String::from("ˊ"), "6"),
        (String::from("˙"), "7"),
        (String::from("ㄚ"), "8"),
        (String::from("ㄞ"), "9"),
        (String::from("ㄢ"), "0"),
        (String::from("ㄦ"), "-"),
        (String::from("ㄆ"), "q"),
        (String::from("ㄊ"), "w"),
        (String::from("ㄍ"), "e"),
        (String::from("ㄐ"), "r"),
        (String::from("ㄔ"), "t"),
        (String::from("ㄗ"), "y"),
        (String::from("ㄧ"), "u"),
        (String::from("ㄛ"), "i"),
        (String::from("ㄟ"), "o"),
        (String::from("ㄣ"), "p"),
        (String::from("_"), ""),
        (String::from("ㄇ"), "a"),
        (String::from("ㄋ"), "s"),
        (String::from("ㄎ"), "d"),
        (String::from("ㄑ"), "f"),
        (String::from("ㄕ"), "g"),
        (String::from("ㄘ"), "h"),
        (String::from("ㄨ"), "j"),
        (String::from("ㄜ"), "k"),
        (String::from("ㄠ"), "l"),
        (String::from("ㄤ"), ";"),
        (String::from("ㄈ"), "z"),
        (String::from("ㄌ"), "x"),
        (String::from("ㄏ"), "c"),
        (String::from("ㄒ"), "v"),
        (String::from("ㄖ"), "b"),
        (String::from("ㄙ"), "n"),
        (String::from("ㄩ"), "m"),
        (String::from("ㄝ"), ","),
        (String::from("ㄡ"), "."),
        (String::from("ㄥ"), "/"),
    ])
});

impl Rime {
    pub fn new() -> anyhow::Result<Self> {
        #[allow(deprecated)]
        let home = std::env::home_dir().expect("homedir does not exists");

        let data_dir = home.join("rime-data");
        let user_dir = home.join("rime-user");
        if !data_dir.exists() {
            std::fs::create_dir(&data_dir)?;
        }
        if !user_dir.exists() {
            std::fs::create_dir(&user_dir)?;
        }

        // Define socket paths *once*
        let keys_path = PathBuf::from("/tmp/rm-rime-keys.sock");
        let candidates_path = PathBuf::from("/tmp/rm-rime-candidates.sock");
        let output_path = PathBuf::from("/tmp/rm-rime-output.sock");
        let backspace_path = PathBuf::from("/tmp/rm-rime-backspace.sock");

        Ok(Self {
            data_dir,
            user_dir,
            keys_path,
            candidates_path,
            backspace_path,
            output_path,
        })
    }

    fn cleanup_socket(path: &PathBuf) -> anyhow::Result<()> {
        if path.exists() {
            let metadata = std::fs::metadata(path)?;
            if metadata.file_type().is_socket() {
                std::fs::remove_file(path)?;
            } else {
                eprintln!("Warning: {} exists but is not a socket.", path.display());
            }
        }
        Ok(())
    }

    #[allow(clippy::redundant_pub_crate)]
    pub async fn start(&self) -> anyhow::Result<()> {
        Self::cleanup_socket(&self.keys_path)?;
        Self::cleanup_socket(&self.candidates_path)?;
        Self::cleanup_socket(&self.output_path)?;
        Self::cleanup_socket(&self.backspace_path)?;

        let keys_listener = UnixListener::bind(&self.keys_path)?;
        let candidates_listener = UnixListener::bind(&self.candidates_path)?;
        let backspace_listener = UnixListener::bind(&self.backspace_path)?;
        let output_listener = UnixListener::bind(&self.output_path)?;

        let data_dir = self.data_dir.to_string_lossy().to_string();
        let user_dir = self.user_dir.to_string_lossy().to_string();

        let mut traits = Traits::new();
        traits.set_shared_data_dir(&data_dir);
        traits.set_user_data_dir(&user_dir);
        traits.set_distribution_name("Rime");
        traits.set_distribution_code_name("Rime");
        traits.set_distribution_version("0.0.0");
        setup(&mut traits);
        initialize(&mut traits);

        set_notification_handler(|t, v| {
            println!("Notification message: {:?}", (t, v));
        });

        let deploy_result = full_deploy_and_wait();
        match deploy_result {
            DeployResult::Success => {
                println!("Deployment done");
            }
            DeployResult::Failure => {
                panic!("Deployment failed");
            }
        }

        let session = create_session()?;
        session.select_schema("iridium_bpmf")?;

        loop {
            let mut output = output_listener.accept().await?.0;

            select! {
                key = keys_listener.accept() => {
                    let mut key = key?.0;
                    let mut buf = String::new();
                    key.read_to_string(&mut buf).await?;
                    if let Some(key) = KEY_MAP.get(&buf) {
                        session.simulate_key_sequence(key)?;
                    } else {
                        session.simulate_key_sequence(&buf)?;
                    }

                }
                candidates = candidates_listener.accept() => {
                    let mut candidates = candidates?.0;
                    let mut buf = String::new();
                    candidates.read_to_string(&mut buf).await?;
                    let index:usize = buf.trim().parse()?;
                    session.select_candidate(index);
                }
                _ = backspace_listener.accept() => {
                    session.backspace();
                }
            };
            if let Some(c) = session.context() {
                let menu = c
                    .menu()
                    .candidates
                    .iter()
                    .map(|x| x.text.into())
                    .collect::<Vec<String>>();

                let preedit = c.composition().preedit;

                if menu.is_empty() {
                    session.simulate_key_sequence(" ")?;
                }

                let output_val = preedit.map_or_else(
                    || {
                        serde_json::json!({
                            "preedit": "",
                            "candidates": menu
                        })
                    },
                    |preedit| {
                        serde_json::json!({
                            "preedit": preedit,
                            "candidates": menu
                        })
                    },
                );

                output.writable().await?;
                output.write_all(output_val.to_string().as_bytes()).await?;
            }
        }
    }

    // Assumes daemon is running
    async fn send_key(&self, key_sequence: &str) -> anyhow::Result<String> {
        let mut stream = UnixStream::connect(&self.keys_path).await?;
        stream.write_all(key_sequence.as_bytes()).await?;
        stream.shutdown().await?;

        let mut output_stream = UnixStream::connect("/tmp/rm-rime-output.sock").await?;
        let mut output = String::new();
        output_stream.read_to_string(&mut output).await?;
        Ok(output)
    }

    // Assumes daemon is running
    async fn send_backspace(&self) -> anyhow::Result<String> {
        let mut stream = UnixStream::connect(&self.backspace_path).await?;
        stream.write_all(b"s").await?;
        stream.shutdown().await?;

        let mut output_stream = UnixStream::connect("/tmp/rm-rime-output.sock").await?;
        let mut output = String::new();
        output_stream.read_to_string(&mut output).await?;
        Ok(output)
    }

    // Assumes daemon is running
    async fn send_candidate(&self, candidate_index: &str) -> anyhow::Result<String> {
        let mut stream = UnixStream::connect(&self.candidates_path).await?;
        stream.write_all(candidate_index.as_bytes()).await?;
        stream.shutdown().await?;

        let mut output_stream = UnixStream::connect("/tmp/rm-rime-output.sock").await?;
        let mut output = String::new();
        output_stream.read_to_string(&mut output).await?;
        Ok(output)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let rime = Rime::new()?;

    while let Some(arg) = args.next() {
        match &*arg {
            "daemon" => {
                rime.start().await?;
            }
            "key" => {
                let Some(contents) = args.next() else {
                    panic!("You must provide the key associated");
                };
                let output = rime.send_key(&contents).await?;
                println!("{output}");
            }
            "candidate" => {
                let Some(contents) = args.next() else {
                    panic!("You must provide the key associated");
                };
                let output = rime.send_candidate(&contents).await?;
                println!("{output}");
            }
            "backspace" => {
                let output = rime.send_backspace().await?;
                println!("{output}");
            }
            _ => {
                panic!("Unsupported.");
            }
        }
    }
    Ok(())
}
