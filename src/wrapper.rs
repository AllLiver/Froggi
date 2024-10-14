use git2::Repository;
#[cfg(unix)]
use nix::{sys::signal::Signal::SIGTERM, unistd::Pid};
use tokio::{process::Command, signal};

const FROGGI_REMOTE_URL: &'static str = "https://github.com/AllLiver/Froggi.git";
const BUILD_TMP_DIR: &'static str = "./tmp/froggi";

#[tokio::main]
async fn main() {
    let mut worker_exe = std::env::current_exe().expect("Failed to get current directory");
    let cwd = std::env::current_dir().expect("Failed to get current working directory");

    worker_exe.pop();

    worker_exe = worker_exe.join("froggi-worker");

    let exit_code: i32 = loop {
        let mut froggi_process = Command::new(worker_exe.clone())
            .spawn()
            .expect("Failed to start froggi-worker");

        tokio::select! {
            // If the process finishes by itself, then handle the exit code
            p = froggi_process.wait() => {
                if let Some(code) = p.expect("Failed to get froggi-worker exit status").code() {
                    match code {
                        // Exit code 10 is to restart
                        10 => {
                            continue;
                        }
                        // Exit code 11 is to update
                        11 => {
                            println!("Cloning {} to {}", FROGGI_REMOTE_URL, BUILD_TMP_DIR);

                            if let Ok(_) = Repository::clone(FROGGI_REMOTE_URL, BUILD_TMP_DIR) {
                                println!("Repository cloned to {}", BUILD_TMP_DIR);
                                println!("Changing current dir to {}", BUILD_TMP_DIR);

                                match std::env::set_current_dir(BUILD_TMP_DIR) {
                                    Ok(_) => {
                                        println!("{}", std::env::current_dir().expect("msg").to_string_lossy());
                                        println!("Compiling update...");

                                        let p = Command::new("cargo").args(&["build", "--release"]).spawn();

                                        match p {
                                            Ok(mut cargo_process) => {
                                                let cargo_process_result = cargo_process.wait().await;

                                                match cargo_process_result {
                                                    Ok(_) => {
                                                        println!("Update compiled successfully!");
                                                        println!("Replacing local froggi-worker with updated froggi-worker...");

                                                        #[cfg(unix)]
                                                        match tokio::fs::rename(format!("./target/release/froggi-worker"), worker_exe.clone()).await {
                                                            Ok(_) => {
                                                                println!("Local froggi-worker replaced, update successful!");
                                                            },
                                                            Err(e) => {
                                                                println!("{} occurred when moving froggi-worker! Update unsuccessful.", e);
                                                            }
                                                        }

                                                        #[cfg(not(unix))]
                                                        match tokio::fs::rename("./target/release/froggi-worker.exe", worker_exe.clone()).await {
                                                            Ok(_) => {
                                                                println!("Local froggi-worker replaced, update successful!");
                                                            },
                                                            Err(e) => {
                                                                println!("{} occurred when moving froggi-worker! Update unsuccessful.", e);
                                                            }
                                                        }
                                                    },
                                                    Err(e) => {
                                                        println!("\"{}\" occurred when compiling froggi! Update unsuccessful.", e);
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                println!("\"{}\" occurred when swpaning cargo build --release! Update unsuccessful.", e);
                                            }
                                        }

                                        println!("Moving back to {}", cwd.to_string_lossy());

                                        if let Err(_) = std::env::set_current_dir(cwd.clone()) {
                                            println!("Failed to move back to {}, please restart froggi manually.", cwd.to_string_lossy());
                                            break 1;
                                        }
                                    },
                                    Err(_) => {
                                        println!("Failed to move to {}", BUILD_TMP_DIR);
                                    }
                                }
                            } else {
                                println!("Failed to clone repository, update unsuccessful.");
                                continue;
                            }

                            println!("Cleaning up...");

                            tokio::fs::remove_dir_all(BUILD_TMP_DIR).await.expect("Failed to remove temp build directory");

                            println!("Restarting froggi...");

                            continue;
                        }
                        _ => {
                            break code;
                        }
                    }
                } else {
                    break 1;
                }
            },
            // If the wrapper recieves a shutdown signal, send a shutdown signal to the child process
            _ = shutdown_signal() => {
                #[cfg(unix)]
                nix::sys::signal::kill(Pid::from_raw(froggi_process.id().expect("Failed to get froggi-worker pid") as i32), SIGTERM).expect("Failed to terminate froggi-worker");

                #[cfg(not(unix))]
                froggi_process.kill().await.expect("Failed to kill child process");

                break 0;
            }
        }
    };

    std::process::exit(exit_code);
}

// Code borrowed from https://github.com/tokio-rs/axum/blob/806bc26e62afc2e0c83240a9e85c14c96bc2ceb3/examples/graceful-shutdown/src/main.rs
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = async {
        let _ = std::future::pending::<()>().await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
