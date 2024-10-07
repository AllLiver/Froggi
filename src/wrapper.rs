use nix::{sys::signal::Signal::SIGTERM, unistd::Pid};
use tokio::{process::Command, signal};

#[tokio::main]
async fn main() {
    let mut worker_exe = std::env::current_exe().expect("Failed to get current directory");

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
                {
                    nix::sys::signal::kill(Pid::from_raw(froggi_process.id().expect("Failed to get froggi-worker pid") as i32), SIGTERM).expect("Failed to terminate froggi-worker");
                }

                #[cfg(not(unix))]
                {
                    froggi_process.kill();
                }

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
